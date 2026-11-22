#!/usr/bin/env python3
"""RtRustMobile Cloud Agent: remote control via Telegram for GitHub Codespaces."""

import asyncio
import glob
import os
import time
from pathlib import Path

from telegram import Update
from telegram.ext import Application, CommandHandler, ContextTypes

MAX_TG_LEN = 3500
CODE_TIMEOUT_SEC = 900
DEFAULT_TIMEOUT_SEC = 1800
CURRENT_MODEL = "gemini-2.5-pro"

MODEL_ALIASES = {
    "pro": "gemini-2.5-pro",
    "flash": "gemini-2.5-flash",
    "auto": "auto",
}


def resolve_workspace() -> Path:
    cwd = Path(os.getcwd())
    if (cwd / "Cargo.toml").exists() and (cwd / "android").exists():
        return cwd
    for candidate in sorted(glob.glob("/workspaces/*")):
        p = Path(candidate)
        if (p / "Cargo.toml").exists() and (p / "android").exists():
            return p
    return cwd


WORKSPACE = resolve_workspace()


def build_env() -> dict[str, str]:
    env = os.environ.copy()
    home = str(Path.home())
    android_home = env.get("ANDROID_HOME") or f"{home}/android-sdk"
    ndk_candidates = sorted(glob.glob(f"{android_home}/ndk/*"))
    android_ndk = env.get("ANDROID_NDK_HOME") or (
        ndk_candidates[-1] if ndk_candidates else f"{android_home}/ndk/27.2.12479018"
    )
    extra_paths = [
        f"{home}/.cargo/bin",
        f"{home}/gradle-8.10.2/bin",
        f"{home}/google-cloud-sdk/bin",
        f"{android_home}/cmdline-tools/latest/bin",
        f"{android_home}/platform-tools",
        f"{android_home}/build-tools/35.0.0",
        f"{android_home}/build-tools/34.0.0",
        "/opt/google-cloud-cli/bin",
    ]
    env["ANDROID_HOME"] = android_home
    env["ANDROID_SDK_ROOT"] = android_home
    env["ANDROID_NDK_HOME"] = android_ndk
    env["PATH"] = ":".join(extra_paths + [env.get("PATH", "")])
    return env


def truncate_msg(text: str, limit: int = MAX_TG_LEN) -> str:
    text = text.strip()
    if not text:
        return "(no output)"
    if len(text) <= limit:
        return text
    return f"...(truncated)\n{text[-limit:]}"


def is_authorized(update: Update, allowed_id: str) -> bool:
    user = update.effective_user
    if not user or not allowed_id:
        return False
    return str(user.id) == allowed_id.strip()


async def run_shell(
    cmd: str, cwd: Path | None = None, timeout: int = DEFAULT_TIMEOUT_SEC
) -> tuple[int, str]:
    proc = await asyncio.create_subprocess_shell(
        cmd,
        cwd=str(cwd or WORKSPACE),
        env=build_env(),
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    try:
        out, _ = await asyncio.wait_for(proc.communicate(), timeout=timeout)
        return proc.returncode or 0, out.decode("utf-8", errors="replace")
    except asyncio.TimeoutError:
        proc.kill()
        await proc.wait()
        return 124, f"Command timed out after {timeout}s."


async def run_shell_streaming(
    cmd: str,
    status_msg,
    header: str,
    timeout: int = CODE_TIMEOUT_SEC,
) -> tuple[int, str]:
    """Run a shell command and periodically update a Telegram status message."""
    proc = await asyncio.create_subprocess_shell(
        cmd,
        cwd=str(WORKSPACE),
        env=build_env(),
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    chunks: list[str] = []
    start = time.monotonic()

    async def reader() -> None:
        assert proc.stdout is not None
        while True:
            line = await proc.stdout.readline()
            if not line:
                break
            chunks.append(line.decode("utf-8", errors="replace"))

    read_task = asyncio.create_task(reader())
    last_edit = 0.0
    try:
        while not read_task.done():
            await asyncio.sleep(1.5)
            elapsed = int(time.monotonic() - start)
            if elapsed >= timeout:
                proc.kill()
                await proc.wait()
                chunks.append(f"\n[Timed out after {timeout}s]")
                break
            if time.monotonic() - last_edit >= 4.0:
                last_edit = time.monotonic()
                tail = "".join(chunks[-12:]).strip()
                preview = truncate_msg(tail, 700) if tail else "Thinking / analyzing workspace..."
                try:
                    await status_msg.edit_text(f"{header}\nElapsed: {elapsed}s\n\n{preview}")
                except Exception:
                    pass
        await read_task
        rc = await proc.wait()
    finally:
        if not read_task.done():
            read_task.cancel()

    return rc or 0, "".join(chunks)


def find_latest_apk() -> Path | None:
    patterns = [
        WORKSPACE / "android/build/outputs/apk/release/*.apk",
        WORKSPACE / "android/build/outputs/apk/debug/*.apk",
        WORKSPACE / "android/app/build/outputs/apk/release/*.apk",
        WORKSPACE / "android/app/build/outputs/apk/debug/*.apk",
    ]
    found: list[Path] = []
    for pat in patterns:
        for p in glob.glob(str(pat)):
            found.append(Path(p))
    if not found:
        return None
    found.sort(key=lambda p: p.stat().st_mtime, reverse=True)
    return found[0]


def help_text() -> str:
    return (
        "RtRustMobile Cloud Agent\n"
        f"Workspace: {WORKSPACE}\n"
        f"Current Model: {CURRENT_MODEL}\n\n"
        "Commands:\n"
        "/start — Check bot status\n"
        "/help — List all commands\n"
        "/model <pro|flash|auto|name> — Switch Gemini model\n"
        "/code <prompt> — Run Gemini CLI (live thinking + code review)\n"
        "/build — Build arm64-v8a + armeabi-v7a & send APK\n"
        "/testlab — Run Firebase Test Lab Robo on project rtrustmobile\n"
        "/sh <command> — Run arbitrary shell command\n"
        "/turnoff [codespace] — Stop bot (or stop Codespace if requested)"
    )


async def cmd_start(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    await update.message.reply_text(help_text())


async def cmd_help(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    await update.message.reply_text(help_text())


async def cmd_model(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    global CURRENT_MODEL
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    arg = " ".join(context.args).strip()
    if not arg:
        await update.message.reply_text(
            f"Current model: {CURRENT_MODEL}\nUsage: /model pro | /model flash | /model auto | /model <name>"
        )
        return
    CURRENT_MODEL = MODEL_ALIASES.get(arg.lower(), arg)
    await update.message.reply_text(f"Switched Gemini model to: {CURRENT_MODEL}")


async def cmd_code(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    prompt = " ".join(context.args).strip()
    if not prompt:
        await update.message.reply_text("Usage: /code <request>")
        return

    model_flag = "" if CURRENT_MODEL == "auto" else f"-m '{CURRENT_MODEL}' "
    header = f"[/code] Model: {CURRENT_MODEL}\nPrompt: {prompt[:300]}"
    status_msg = await update.message.reply_text(f"{header}\n\nThinking...")

    safe_prompt = prompt.replace("'", "'\"'\"'")
    cmd = f"gemini {model_flag}-y -p '{safe_prompt}'"
    rc, out = await run_shell_streaming(cmd, status_msg, header, timeout=CODE_TIMEOUT_SEC)
    status = "OK" if rc == 0 else f"FAILED (exit {rc})"

    await update.message.reply_text(
        f"[/code {status} | model: {CURRENT_MODEL}]\n\n{truncate_msg(out)}"
    )

    _, diff_stat = await run_shell("git status -s && echo '--- DIFF ---' && git diff --stat")
    _, diff_body = await run_shell("git diff --no-color")
    review_text = f"[Code Review]\n{diff_stat.strip()}"
    if diff_body.strip():
        review_text += f"\n\n{diff_body.strip()}"
    await update.message.reply_text(truncate_msg(review_text))


async def cmd_build(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    await update.message.reply_text("Building arm64-v8a + armeabi-v7a (.so) & Android APK...")
    gradle_cmd = "./gradlew" if (WORKSPACE / "android/gradlew").exists() else "gradle"
    cmd = (
        "cargo ndk -P 26 -t arm64-v8a -t armeabi-v7a -o android/jniLibs build --profile mobile-release --lib && "
        f"(cd android && {gradle_cmd} assembleRelease assembleDebug --no-daemon)"
    )
    rc, out = await run_shell(cmd)
    if rc != 0:
        await update.message.reply_text(f"[/build FAILED exit {rc}]\n\n{truncate_msg(out)}")
        return

    apk = None
    for candidate in [
        WORKSPACE / "android/build/outputs/apk/release/RtRustMobile-release.apk",
        WORKSPACE / "android/build/outputs/apk/release/app-release.apk",
        WORKSPACE / "android/build/outputs/apk/debug/RtRustMobile-debug.apk",
    ]:
        if candidate.exists():
            apk = candidate
            break
    if apk is None:
        apk = find_latest_apk()

    if apk is None or not apk.exists():
        await update.message.reply_text(f"Build succeeded but no APK found.\n\n{truncate_msg(out)}")
        return

    size_mb = apk.stat().st_size / (1024 * 1024)
    await update.message.reply_text(f"Build OK! Uploading {apk.name} ({size_mb:.1f} MB)...")
    with apk.open("rb") as f:
        await update.message.reply_document(document=f, filename=apk.name)


async def cmd_testlab(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    apk = find_latest_apk()
    if apk is None:
        await update.message.reply_text("No APK found. Run /build first.")
        return

    await update.message.reply_text(f"Submitting {apk.name} to Firebase Test Lab (rtrustmobile)...")
    cmd = (
        "gcloud config set project rtrustmobile && "
        f"gcloud firebase test android run --type robo --app '{apk}' "
        "--device model=SC-51E,version=36,locale=en,orientation=portrait "
        "--device model=OP5759L1,version=34,locale=en,orientation=portrait "
        "--timeout 3m"
    )
    rc, out = await run_shell(cmd)
    status = "PASSED" if rc == 0 else f"FAILED (exit {rc})"
    await update.message.reply_text(f"[/testlab {status}]\n\n{truncate_msg(out)}")


async def cmd_sh(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    raw_cmd = " ".join(context.args).strip()
    if not raw_cmd:
        await update.message.reply_text("Usage: /sh <shell command>")
        return
    rc, out = await run_shell(raw_cmd)
    await update.message.reply_text(f"[/sh exit {rc}]\n\n{truncate_msg(out)}")


async def cmd_turnoff(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    arg = " ".join(context.args).strip().lower()
    cs_name = os.environ.get("CODESPACE_NAME", "").strip()
    if arg == "codespace" and cs_name:
        await update.message.reply_text(f"Stopping Codespace {cs_name} and bot...")
        await run_shell(f"gh codespace stop -c '{cs_name}'")
    else:
        await update.message.reply_text("Stopping Telegram bot...")
    os._exit(0)


def main() -> None:
    token = os.environ.get("TG_BOT_TOKEN", "").strip()
    if not token:
        raise SystemExit("TG_BOT_TOKEN is not set in environment.")
    app = Application.builder().token(token).build()
    app.add_handler(CommandHandler("start", cmd_start))
    app.add_handler(CommandHandler("help", cmd_help))
    app.add_handler(CommandHandler("model", cmd_model))
    app.add_handler(CommandHandler("code", cmd_code))
    app.add_handler(CommandHandler("build", cmd_build))
    app.add_handler(CommandHandler("testlab", cmd_testlab))
    app.add_handler(CommandHandler("sh", cmd_sh))
    app.add_handler(CommandHandler("turnoff", cmd_turnoff))
    app.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()
