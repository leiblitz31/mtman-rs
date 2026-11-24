#!/bin/bash
set -e
cd "$(dirname "$0")"

BT=/home/hatch/android-sdk/build-tools/36.0.0
PLATFORM=/home/hatch/android-sdk/platforms/android-34/android.jar
APKDIR=app/src/main

echo "==> 1. Rust (aarch64)"
cargo build --target aarch64-linux-android --release --features android 2>/dev/null
mkdir -p $APKDIR/jniLibs/arm64-v8a
cp ../target/aarch64-linux-android/release/libmtman.so $APKDIR/jniLibs/arm64-v8a/

echo "==> 2. Resources"
rm -rf build && mkdir -p build/gen build/classes
$BT/aapt2 compile --dir $APKDIR/res -o build/res.zip
$BT/aapt2 link -o build/base.apk -I $PLATFORM \
  --manifest $APKDIR/AndroidManifest.xml \
  build/res.zip --java build/gen \
  --min-sdk-version 21 --target-sdk-version 34 \
  --version-code 1 --version-name 0.1.0

echo "==> 3. Java"
/usr/bin/javac -source 8 -target 8 -classpath $PLATFORM -d build/classes \
  $(find $APKDIR/java -name "*.java") 2>&1 | grep -v "^Note:" || true

echo "==> 4. dex"
CLASSES=$(find build/classes -name "*.class" | tr '\n' ' ')
echo "  classes: $(echo $CLASSES | wc -w)"
$BT/d8 --lib $PLATFORM --min-api 21 --output build/ $CLASSES

echo "==> 5. Package"
cd build
mkdir -p lib/arm64-v8a
cp ../$APKDIR/jniLibs/arm64-v8a/libmtman.so lib/arm64-v8a/
zip -q base.apk classes.dex lib/arm64-v8a/libmtman.so
cd ..

echo "==> 6. Align + sign"
$BT/zipalign -f -p 4 build/base.apk build/aligned.apk
KEY=/home/hatch/keystores/debug.keystore
[ -f "$KEY" ] || keytool -genkeypair -v -keystore $KEY -storepass android \
  -alias androiddebugkey -keypass android -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=Android Debug,O=Android,C=US" 2>/dev/null
$BT/apksigner sign --ks $KEY --ks-pass pass:android --ks-key-alias androiddebugkey \
  --key-pass pass:android --out build/MTManager.apk build/aligned.apk

echo "==> DONE"
ls -la build/MTManager.apk
