#!/bin/bash
set -e
cd "$(dirname "$0")"

BT=/home/hatch/android-sdk/build-tools/36.0.0
PLATFORM=/home/hatch/android-sdk/platforms/android-34/android.jar
APKDIR=app/src/main

# ABI -> rust target
build_rust() {
  local abi=$1 target=$2
  echo "==> Rust: $abi ($target)"
  (cd .. && cargo build --target $target --release --features android 2>/dev/null)
  mkdir -p $APKDIR/jniLibs/$abi
  cp ../target/$target/release/libmtman.so $APKDIR/jniLibs/$abi/
}

build_rust arm64-v8a aarch64-linux-android
build_rust armeabi-v7a armv7-linux-androideabi
build_rust x86 i686-linux-android
build_rust x86_64 x86_64-linux-android

echo "==> Resources"
rm -rf build && mkdir -p build/gen build/classes
$BT/aapt2 compile --dir $APKDIR/res -o build/res.zip
$BT/aapt2 link -o build/base.apk -I $PLATFORM \
  --manifest $APKDIR/AndroidManifest.xml \
  build/res.zip --java build/gen \
  --min-sdk-version 21 --target-sdk-version 34 \
  --version-code 1 --version-name 0.1.0

echo "==> Java"
/usr/bin/javac -source 8 -target 8 -classpath $PLATFORM -d build/classes \
  $(find $APKDIR/java -name "*.java") 2>&1 \
  | grep -v "bootstrap class path\|source value 8\|target value 8\|To suppress" || true

echo "==> dex"
$BT/d8 --lib $PLATFORM --min-api 21 --output build/ \
  $(find build/classes -name "*.class")

echo "==> Package"
cd build
mkdir -p lib
for abi in arm64-v8a armeabi-v7a x86 x86_64; do
  mkdir -p lib/$abi
  cp ../$APKDIR/jniLibs/$abi/libmtman.so lib/$abi/
done
zip -q base.apk classes.dex $(find lib -type f)
cd ..

echo "==> Align + sign"
$BT/zipalign -f -p 4 build/base.apk build/aligned.apk
KEY=/home/hatch/keystores/debug.keystore
[ -f "$KEY" ] || keytool -genkeypair -v -keystore $KEY -storepass android \
  -alias androiddebugkey -keypass android -keyalg RSA -keysize 2048 -validity 10000 \
  -dname "CN=Android Debug,O=Android,C=US" 2>/dev/null
$BT/apksigner sign --ks $KEY --ks-pass pass:android --ks-key-alias androiddebugkey \
  --key-pass pass:android --out build/MTManager.apk build/aligned.apk

echo "==> DONE"
ls -la build/MTManager.apk
