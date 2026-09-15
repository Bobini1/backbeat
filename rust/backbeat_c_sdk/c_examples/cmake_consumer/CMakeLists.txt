find_package(Backbeat CONFIG REQUIRED)
find_package(Backbeat CONFIG REQUIRED)

get_filename_component(consumer_name "${CMAKE_CURRENT_BINARY_DIR}" NAME)
add_executable(${consumer_name} ../cpp_link.cpp)
target_link_libraries(${consumer_name} PRIVATE Backbeat::Backbeat)
add_test(NAME ${consumer_name} COMMAND ${consumer_name})
