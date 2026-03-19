cmake_minimum_required(VERSION 3.8)

project(IbDllHijack)

# header-only
add_library(IbDllHijack INTERFACE)
target_include_directories(IbDllHijack INTERFACE include)

# test
if(CMAKE_PROJECT_NAME STREQUAL PROJECT_NAME)
    option(BUILD_TESTING "Build the testing tree." OFF)
    include(CTest)
    if(BUILD_TESTING)
        find_package(Boost REQUIRED COMPONENTS unit_test_framework)

        add_library(export_forward MODULE test/export_forward.c)
        target_link_libraries(export_forward PRIVATE IbDllHijack)

        add_executable(IbDllHijack_test test/test.cpp)
        target_link_libraries(IbDllHijack_test
            PRIVATE Boost::unit_test_framework
            # export_forward
        )
        target_compile_features(IbDllHijack_test PRIVATE cxx_std_17)
    endif()
endif()