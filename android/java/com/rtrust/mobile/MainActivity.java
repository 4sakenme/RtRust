package com.rtrust.mobile;

import android.app.NativeActivity;
import android.content.Context;
import android.os.Build;
import android.os.Bundle;
import android.text.InputType;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowManager;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputConnection;
import android.view.inputmethod.InputMethodManager;

public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("rtrust_mobile");
    }

    private static native void nativeOnImeKey(int kind, int codepoint);

    private View imeView;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        hideSystemUI();

        imeView = new View(this) {
            @Override
            public boolean onCheckIsTextEditor() {
                return true;
            }

            @Override
            public InputConnection onCreateInputConnection(EditorInfo outAttrs) {
                outAttrs.inputType = InputType.TYPE_NULL;
                outAttrs.imeOptions = EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_ACTION_DONE;
                return new BaseInputConnection(this, false) {
                    @Override
                    public boolean commitText(CharSequence text, int newCursorPosition) {
                        sendChars(text);
                        return true;
                    }

                    @Override
                    public boolean setComposingText(CharSequence text, int newCursorPosition) {
                        sendChars(text);
                        return true;
                    }

                    @Override
                    public boolean deleteSurroundingText(int beforeLength, int afterLength) {
                        for (int i = 0; i < beforeLength; i++) {
                            nativeOnImeKey(1, 0);
                        }
                        return true;
                    }

                    @Override
                    public boolean sendKeyEvent(KeyEvent event) {
                        if (event.getAction() == KeyEvent.ACTION_DOWN) {
                            int code = event.getKeyCode();
                            if (code == KeyEvent.KEYCODE_DEL) {
                                nativeOnImeKey(1, 0);
                                return true;
                            }
                            if (code == KeyEvent.KEYCODE_ENTER || code == KeyEvent.KEYCODE_NUMPAD_ENTER) {
                                nativeOnImeKey(2, 0);
                                return true;
                            }
                            int u = event.getUnicodeChar();
                            if (u != 0 && (u & KeyEvent.META_CTRL_ON) == 0) {
                                nativeOnImeKey(0, u);
                                return true;
                            }
                        } else if (event.getAction() == KeyEvent.ACTION_MULTIPLE) {
                            String chars = event.getCharacters();
                            if (chars != null) {
                                sendChars(chars);
                                return true;
                            }
                        }
                        return super.sendKeyEvent(event);
                    }
                };
            }
        };
        imeView.setFocusable(true);
        imeView.setFocusableInTouchMode(true);
        addContentView(imeView, new ViewGroup.LayoutParams(1, 1));
    }

    private static void sendChars(CharSequence text) {
        if (text == null) return;
        for (int i = 0; i < text.length(); ) {
            int cp = Character.codePointAt(text, i);
            i += Character.charCount(cp);
            if (cp == '\n' || cp == '\r') {
                nativeOnImeKey(2, 0);
            } else {
                nativeOnImeKey(0, cp);
            }
        }
    }

    @SuppressWarnings("unused")
    public void setSoftInput(final boolean show) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
                if (imm == null || imeView == null) return;
                if (show) {
                    imeView.requestFocus();
                    imm.showSoftInput(imeView, InputMethodManager.SHOW_IMPLICIT);
                } else {
                    imm.hideSoftInputFromWindow(imeView.getWindowToken(), 0);
                    imeView.clearFocus();
                }
            }
        });
    }

    @Override
    public void onWindowFocusChanged(boolean hasFocus) {
        super.onWindowFocusChanged(hasFocus);
        if (hasFocus) {
            hideSystemUI();
        }
    }

    private void hideSystemUI() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            getWindow().getAttributes().layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES;
        }
        getWindow().getDecorView().setSystemUiVisibility(
            View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
            | View.SYSTEM_UI_FLAG_LAYOUT_STABLE
            | View.SYSTEM_UI_FLAG_LAYOUT_HIDE_NAVIGATION
            | View.SYSTEM_UI_FLAG_LAYOUT_FULLSCREEN
            | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
            | View.SYSTEM_UI_FLAG_FULLSCREEN
        );
    }
}
