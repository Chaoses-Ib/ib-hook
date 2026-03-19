#define BOOST_TEST_MODULE test
#include <boost/test/unit_test.hpp>

#include <string_view>
#include <Windows.h>

BOOST_AUTO_TEST_SUITE(ExportTest)

    BOOST_AUTO_TEST_CASE(ExportForward) {
        HMODULE h = LoadLibraryW(L"export_forward.dll");
        BOOST_REQUIRE(h);

        // Boost.Test cannot output wchar_t
        auto VerLanguageNameA = (decltype(&::VerLanguageNameA))GetProcAddress(h, "VerLanguageNameA");
        BOOST_REQUIRE(VerLanguageNameA);

        char buf1[64], buf2[64];
        std::string_view sv1{ buf1, ::VerLanguageNameA(0x040A, buf1, std::size(buf1)) };
        std::string_view sv2{ buf2, VerLanguageNameA(0x040A, buf2, std::size(buf2)) };
        BOOST_TEST_MESSAGE("export_forward:");
        BOOST_TEST_MESSAGE("    " << buf2);
        BOOST_CHECK(sv1 == sv2);
    }

BOOST_AUTO_TEST_SUITE_END()