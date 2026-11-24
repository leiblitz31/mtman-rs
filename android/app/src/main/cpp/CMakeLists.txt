cmake_minimum_required(VERSION 3.22.1)
project(mtman)

add_library(mtman SHARED
    IMPORTED
)

set_target_properties(mtman PROPERTIES
    IMPORTED_LOCATION ${CMAKE_SOURCE_DIR}/../../../target/aarch64-linux-android/release/libmtman.so
)
