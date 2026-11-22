#!/usr/bin/env python3
"""RtRustMobile Cloud Agent: remote control via Telegram for GitHub Codespaces."""

import asyncio
import glob
import os
import subprocess
from pathlib import Path

from telegram import Update
from telegram.ext import Application, CommandHandler, ContextTypes

MAX_TG_LEN = 3500


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


async def run_shell(cmd: str, cwd: Path | None = None) -> tuple[int, str]:
    proc = await asyncio.create_subprocess_shell(
        cmd,
        cwd=str(cwd or WORKSPACE),
        env=build_env(),
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.STDOUT,
    )
    out, _ = await proc.communicate()
    return proc.returncode or 0, out.decode("utf-8", errors="replace")


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


async def cmd_start(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    await update.message.reply_text(
        "RtRustMobile Cloud Agent ready!\n"
        f"Workspace: {WORKSPACE}\n\n"
        "Commands:\n"
        "/code <prompt> — Run Gemini CLI to edit code\n"
        "/build — Build arm64-v8a + armeabi-v7a & send APK\n"
        "/testlab — Run Firebase Test Lab Robo on project rtrustmobile\n"
        "/sh <command> — Run shell command"
    )


async def cmd_code(update: Update, context: ContextTypes.DEFAULT_TYPE) -> None:
    allowed = os.environ.get("TG_ALLOWED_ID", "")
    if not is_authorized(update, allowed):
        return
    prompt = " ".join(context.args).strip()
    if not prompt:
        await update.message.reply_text("Usage: /code <request>")
        return
    await update.message.reply_text(f"Running Gemini CLI for:\n{prompt[:400]}")
    safe_prompt = subprocess.list2cmdline([prompt]) if os.name == "nt" else prompt.replace("'", "'\"'\"'")
    cmd = f"gemini -y -p '{safe_prompt}'"
    rc, out = await run_shell(cmd)
    status = "OK" if rc == 0 else f"FAILED (exit {rc})"
    await update.message.reply_text(f"[/code {status}]\n\n{truncate_msg(out)}")


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

    # Prefer signed release APK if present, else debug APK (signed & directly installable)
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


def main() -> None:
    token = os.environ.get("TG_BOT_TOKEN", "").strip()
    if not token:
        raise SystemExit("TG_BOT_TOKEN is not set in environment.")
    app = Application.builder().token(token).build()
    app.add_handler(CommandHandler("start", cmd_start))
    app.add_handler(CommandHandler("code", cmd_code))
    app.add_handler(CommandHandler("build", cmd_build))
    app.add_handler(CommandHandler("testlab", cmd_testlab))
    app.add_handler(CommandHandler("sh", cmd_sh))
    app.run_polling(allowed_updates=Update.ALL_TYPES)


if __name__ == "__main__":
    main()
