cmake_minimum_required(VERSION 3.12)
project(cqlib_c_example LANGUAGES C)

if(NOT CMAKE_BUILD_TYPE)
    set(CMAKE_BUILD_TYPE Debug)
endif()

if(CMAKE_BUILD_TYPE MATCHES "Release")
    set(CARGO_PROFILE "release")
    set(CARGO_FLAGS "--release")
else()
    set(CARGO_PROFILE "debug")
    set(CARGO_FLAGS "")
endif()

if(WIN32)
    set(RUST_LIB_NAME "binding_c.lib")
else()
    set(RUST_LIB_NAME "libbinding_c.a")
endif()

if(DEFINED ENV{CARGO_TARGET_DIR})
    set(RUST_TARGET_DIR "$ENV{CARGO_TARGET_DIR}")
else()
    set(RUST_TARGET_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../../target")
endif()

set(RUST_LIB "${RUST_TARGET_DIR}/${CARGO_PROFILE}/${RUST_LIB_NAME}")

find_program(CARGO_EXECUTABLE cargo REQUIRED)

add_custom_target(cargo_build_binding_c
    COMMAND ${CARGO_EXECUTABLE} build --package binding-c ${CARGO_FLAGS}
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}/../.."
    COMMENT "Building Rust library binding-c (${CARGO_PROFILE})..."
    VERBATIM
)

add_executable(cqlib_c_example examples/main.c)
add_dependencies(cqlib_c_example cargo_build_binding_c)

target_include_directories(cqlib_c_example PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/include")
target_link_libraries(cqlib_c_example PRIVATE ${RUST_LIB})

if(APPLE)
    enable_language(CXX)
    set_target_properties(cqlib_c_example PROPERTIES LINKER_LANGUAGE CXX)
    target_link_libraries(cqlib_c_example PRIVATE c++ "-framework CoreFoundation")
endif()

if(UNIX AND NOT APPLE)
    target_link_libraries(cqlib_c_example PRIVATE pthread dl m)
endif()

if(WIN32)
    target_link_libraries(cqlib_c_example PRIVATE ntdll ws2_32 bcrypt userenv advapi32)
endif()
