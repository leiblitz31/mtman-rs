package com.mtman;

import android.app.Activity;
import android.os.Bundle;
import android.view.View;
import android.widget.AdapterView;
import android.widget.ArrayAdapter;
import android.widget.Button;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ListView;
import android.widget.TextView;
import android.widget.Toast;

import org.json.JSONArray;
import org.json.JSONObject;

import java.io.File;
import java.util.ArrayList;
import java.util.Collections;
import java.util.Comparator;
import java.util.List;

/**
 * File browser + APK/DEX/ARSC inspector.
 * All heavy lifting (listing, parsing) happens in Rust via libmtman.so.
 */
public class MainActivity extends Activity {
    static {
        System.loadLibrary("mtman");
    }

    private EditText pathBar;
    private TextView statusBar;
    private ListView listView;
    private ArrayAdapter<String> adapter;
    private final List<File> entries = new ArrayList<>();

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        root.setPadding(dp(12), dp(12), dp(12), dp(12));

        pathBar = new EditText(this);
        pathBar.setText("/sdcard");
        pathBar.setSingleLine(true);
        root.addView(pathBar);

        LinearLayout row = new LinearLayout(this);
        row.setOrientation(LinearLayout.HORIZONTAL);

        Button go = new Button(this);
        go.setText("Go");
        go.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                navigate(pathBar.getText().toString());
            }
        });
        row.addView(go);

        Button up = new Button(this);
        up.setText("Up");
        up.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                goUp();
            }
        });
        row.addView(up);

        Button inspect = new Button(this);
        inspect.setText("Inspect");
        inspect.setOnClickListener(new View.OnClickListener() {
            @Override
            public void onClick(View v) {
                inspectPath(pathBar.getText().toString());
            }
        });
        row.addView(inspect);

        root.addView(row);

        statusBar = new TextView(this);
        statusBar.setTextSize(12);
        statusBar.setText("Ready");
        root.addView(statusBar);

        listView = new ListView(this);
        adapter = new ArrayAdapter<String>(this,
                android.R.layout.simple_list_item_1, new ArrayList<String>());
        listView.setAdapter(adapter);
        listView.setOnItemClickListener(new AdapterView.OnItemClickListener() {
            @Override
            public void onItemClick(AdapterView<?> parent, View view, int position, long id) {
                onEntryClicked(position);
            }
        });
        root.addView(listView, new LinearLayout.LayoutParams(
                LinearLayout.LayoutParams.MATCH_PARENT, 0, 1f));

        setContentView(root);

        if (savedInstanceState != null) {
            String p = savedInstanceState.getString("path");
            if (p != null) pathBar.setText(p);
        }
        navigate(pathBar.getText().toString());
    }

    @Override
    protected void onSaveInstanceState(Bundle outState) {
        outState.putString("path", pathBar.getText().toString());
        super.onSaveInstanceState(outState);
    }

    private int dp(int v) {
        return (int) (v * getResources().getDisplayMetrics().density);
    }

    private void setStatus(String s) {
        statusBar.setText(s);
    }

    /** List a directory using Rust (nativeListDir). */
    private void navigate(String path) {
        File dir = new File(path);
        if (!dir.isDirectory()) {
            setStatus("Not a directory: " + path);
            return;
        }
        String json = listDir(path);
        entries.clear();
        adapter.clear();
        try {
            JSONArray arr = new JSONArray(json);
            for (int i = 0; i < arr.length(); i++) {
                JSONObject o = arr.getJSONObject(i);
                if (o.has("error")) {
                    setStatus("Error: " + o.getString("error"));
                    return;
                }
                File f = new File(dir, o.getString("name"));
                entries.add(f);
                adapter.add((o.getBoolean("dir") ? "[D] " : "[F] ")
                        + o.getString("name")
                        + (o.getBoolean("dir") ? "" : "  (" + o.getLong("size") + " B)"));
            }
        } catch (Exception e) {
            setStatus("Parse error: " + e);
            return;
        }
        Collections.sort(entries, new Comparator<File>() {
            @Override
            public int compare(File a, File b) {
                if (a.isDirectory() != b.isDirectory()) {
                    return a.isDirectory() ? -1 : 1;
                }
                return a.getName().compareToIgnoreCase(b.getName());
            }
        });
        pathBar.setText(path);
        setStatus(entries.size() + " entries");
    }

    private void goUp() {
        String p = pathBar.getText().toString();
        File parent = new File(p).getParentFile();
        if (parent != null) {
            navigate(parent.getAbsolutePath());
        } else {
            setStatus("Already at root");
        }
    }

    private void onEntryClicked(int position) {
        if (position < 0 || position >= entries.size()) return;
        File f = entries.get(position);
        if (f.isDirectory()) {
            navigate(f.getAbsolutePath());
        } else {
            pathBar.setText(f.getAbsolutePath());
            setStatus("Selected: " + f.getName());
        }
    }

    /** Run the right Rust inspector for a file. */
    private void inspectPath(String p) {
        File f = new File(p);
        if (!f.isFile()) {
            setStatus("Select a file to inspect");
            return;
        }
        String name = f.getName().toLowerCase();
        String json;
        if (name.endsWith(".apk")) {
            json = apkInfo(p);
        } else if (name.endsWith(".dex")) {
            json = dexInfo(p);
        } else if (name.equals("resources.arsc")) {
            json = arscInfo(p);
        } else {
            setStatus("No inspector for " + f.getName());
            return;
        }
        setStatus(json.replace("\n", " "));
        Toast.makeText(this, json, Toast.LENGTH_LONG).show();
    }

    // ---- Rust bindings live in Jni (see Jni.java) ----

    private static String greeting() { return Jni.greeting(); }
    private static String listDir(String p) { return Jni.listDir(p); }
    private static String apkInfo(String p) { return Jni.apkInfo(p); }
    private static String dexInfo(String p) { return Jni.dexInfo(p); }
    private static String arscInfo(String p) { return Jni.arscInfo(p); }
}