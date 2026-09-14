# This is entirely AI generated
# I don't understand CMake at all. And probably neither do you
# But if you do, please review this and let me know - zk
include_guard(GLOBAL)

get_filename_component(_backbeat_prefix "${CMAKE_CURRENT_LIST_DIR}/../../.." ABSOLUTE)

if(WIN32)
    set(_backbeat_library "${_backbeat_prefix}/lib/backbeat_c_sdk.lib")
    set(_backbeat_sqlite_library "${_backbeat_prefix}/lib/sqlite3.lib")
    set(_backbeat_system_libraries
        advapi32 bcrypt kernel32 ntdll ole32 shell32 userenv ws2_32 msvcrt)
else()
    set(_backbeat_library "${_backbeat_prefix}/lib/libbackbeat_c_sdk.a")
    set(_backbeat_sqlite_library "${_backbeat_prefix}/lib/libsqlite3.a")
    if(APPLE)
        set(_backbeat_system_libraries
            "-framework Security"
            "-framework SystemConfiguration"
            "-framework CoreFoundation"
            iconv)
    else()
        set(_backbeat_system_libraries
            ssl crypto gcc_s util rt pthread m dl c)
    endif()
endif()

set(_backbeat_include_dir "${_backbeat_prefix}/include")

if(NOT EXISTS "${_backbeat_library}")
    set(Backbeat_FOUND FALSE)
    set(Backbeat_NOT_FOUND_MESSAGE "Backbeat archive not found: ${_backbeat_library}")
    return()
endif()

if(NOT EXISTS "${_backbeat_include_dir}/backbeat.h")
    set(Backbeat_FOUND FALSE)
    set(Backbeat_NOT_FOUND_MESSAGE "Backbeat header not found: ${_backbeat_include_dir}/backbeat.h")
    return()
endif()

add_library(Backbeat::Backbeat STATIC IMPORTED)
set_target_properties(Backbeat::Backbeat PROPERTIES
    IMPORTED_LOCATION "${_backbeat_library}"
    INTERFACE_INCLUDE_DIRECTORIES "${_backbeat_include_dir}"
    INTERFACE_LINK_LIBRARIES "${_backbeat_system_libraries}"
)

if(EXISTS "${_backbeat_sqlite_library}")
    add_library(Backbeat::SQLite STATIC IMPORTED)
    set_target_properties(Backbeat::SQLite PROPERTIES
        IMPORTED_LOCATION "${_backbeat_sqlite_library}"
        INTERFACE_INCLUDE_DIRECTORIES "${_backbeat_include_dir}"
    )
endif()

set(Backbeat_FOUND TRUE)

unset(_backbeat_include_dir)
unset(_backbeat_library)
unset(_backbeat_prefix)
unset(_backbeat_sqlite_library)
unset(_backbeat_system_libraries)
