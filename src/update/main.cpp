#include <iostream>
#include <string>
#include <vector>
#include <cstdlib>
#include <sstream>

#include "windows.cpp"
#include "linux.cpp"
#include "updater.cpp"

int main(int argc, char* argv[]) {
    for (int i = 1; i < argc; ++i) {
        if (std::string(argv[i]) == "--update") {
            runUpdater();
            return 0;
        }
    }
    std::cout << "Usage: updater --update\n";
    return 0;
}