package com.rtrust.mobile;

import android.app.NativeActivity;
import android.content.Context;
import android.graphics.Color;
import android.os.Build;
import android.os.Bundle;
import android.text.Editable;
import android.text.InputType;
import android.text.TextWatcher;
import android.view.KeyEvent;
import android.view.View;
import android.view.ViewGroup;
import android.view.WindowManager;
import android.view.inputmethod.BaseInputConnection;
import android.view.inputmethod.EditorInfo;
import android.view.inputmethod.InputMethodManager;
import android.widget.EditText;
import android.widget.TextView;

public class MainActivity extends NativeActivity {
    static {
        System.loadLibrary("rtrust_mobile");
    }

    private static native void nativeInitActivity(Object activity);
    private static native void nativeOnImeKey(int kind, int codepoint);
    private static native void nativeOnImeHeight(int heightPx);

    private EditText imeInput;
    private String lastText = "";
    private boolean updatingText = false;
    private int lastKbHeight = -1;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        nativeInitActivity(this);
        hideSystemUI();

        imeInput = new EditText(this);
        imeInput.setBackgroundColor(Color.TRANSPARENT);
        imeInput.setTextColor(Color.TRANSPARENT);
        imeInput.setCursorVisible(false);
        imeInput.setAlpha(0f);
        imeInput.setInputType(
            InputType.TYPE_CLASS_TEXT
                | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
                | InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
        );
        imeInput.setImeOptions(EditorInfo.IME_FLAG_NO_EXTRACT_UI | EditorInfo.IME_ACTION_DONE);
        imeInput.setSingleLine(true);

        imeInput.addTextChangedListener(new TextWatcher() {
            @Override
            public void beforeTextChanged(CharSequence s, int start, int count, int after) {}

            @Override
            public void onTextChanged(CharSequence s, int start, int before, int count) {}

            @Override
            public void afterTextChanged(Editable s) {
                if (updatingText) return;
                String next = s.toString();
                int common = 0;
                int minLen = Math.min(lastText.length(), next.length());
                while (common < minLen && lastText.charAt(common) == next.charAt(common)) {
                    common++;
                }
                if (common > 0 && Character.isHighSurrogate(lastText.charAt(common - 1))) {
                    common--;
                }
                int removed = lastText.codePointCount(common, lastText.length());
                for (int i = 0; i < removed; i++) {
                    nativeOnImeKey(1, 0);
                }
                if (common < next.length()) {
                    sendChars(next.substring(common));
                }
                if (BaseInputConnection.getComposingSpanStart(s) < 0 && s.length() > 0) {
                    updatingText = true;
                    s.clear();
                    lastText = "";
                    updatingText = false;
                } else {
                    lastText = next;
                }
            }
        });

        imeInput.setOnKeyListener(new View.OnKeyListener() {
            @Override
            public boolean onKey(View v, int keyCode, KeyEvent event) {
                if (event.getAction() == KeyEvent.ACTION_DOWN) {
                    if (keyCode == KeyEvent.KEYCODE_DEL && imeInput.getText().length() == 0) {
                        nativeOnImeKey(1, 0);
                        return true;
                    }
                    if (keyCode == KeyEvent.KEYCODE_ENTER || keyCode == KeyEvent.KEYCODE_NUMPAD_ENTER) {
                        nativeOnImeKey(2, 0);
                        return true;
                    }
                }
                return false;
            }
        });

        imeInput.setOnEditorActionListener(new TextView.OnEditorActionListener() {
            @Override
            public boolean onEditorAction(TextView v, int actionId, KeyEvent event) {
                nativeOnImeKey(2, 0);
                return true;
            }
        });

        addContentView(imeInput, new ViewGroup.LayoutParams(1, 1));

        final View decor = getWindow().getDecorView();
        decor.getViewTreeObserver().addOnGlobalLayoutListener(new android.view.ViewTreeObserver.OnGlobalLayoutListener() {
            @Override
            public void onGlobalLayout() {
                android.graphics.Rect r = new android.graphics.Rect();
                decor.getWindowVisibleDisplayFrame(r);
                int rootH = decor.getRootView().getHeight();
                int kb = rootH - r.bottom;
                int effective = (kb > rootH * 0.15) ? kb : 0;
                if (effective != lastKbHeight) {
                    lastKbHeight = effective;
                    nativeOnImeHeight(effective);
                }
            }
        });
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
                if (imm == null || imeInput == null) return;
                if (show) {
                    updatingText = true;
                    imeInput.setText("");
                    lastText = "";
                    updatingText = false;
                    imeInput.requestFocus();
                    imeInput.post(new Runnable() {
                        @Override
                        public void run() {
                            imm.restartInput(imeInput);
                            imm.showSoftInput(imeInput, 0);
                        }
                    });
                } else {
                    imm.hideSoftInputFromWindow(imeInput.getWindowToken(), 0);
                    imeInput.clearFocus();
                    updatingText = true;
                    imeInput.setText("");
                    lastText = "";
                    updatingText = false;
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
