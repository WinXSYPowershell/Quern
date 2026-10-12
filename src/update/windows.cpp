#ifdef _WIN32
#include <windows.h>
#include <wincrypt.h>
#pragma comment(lib, "crypt32.lib")

std::string Base64Encode(const std::string& input) {
    DWORD cbEncoded = 0;
    if (!CryptBinaryToStringA((const BYTE*)input.c_str(), (DWORD)input.size(), CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, NULL, &cbEncoded)) {
        return "";
    }
    std::string output(cbEncoded, 0);
    if (!CryptBinaryToStringA((const BYTE*)input.c_str(), (DWORD)input.size(), CRYPT_STRING_BASE64 | CRYPT_STRING_NOCRLF, &output[0], &cbEncoded)) {
        return "";
    }
    return output;
}
#endif