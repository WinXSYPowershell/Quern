void runUpdater() {
    std::cout << "========================================\n";
    std::cout << "       Quern Auto-Updater Started\n";
    std::cout << "========================================\n";

#ifdef _WIN32
    std::cout << "[Updater] Detected Windows OS.\n";
    std::cout << "[Updater] Executing update command...\n";

    std::string psScript =
        "git ls-remote --tags https://gitcode.com/2401_86702825/Quern | "
        "ForEach-Object { $_ -split '\\s+' } | "
        "Where-Object { $_ -match 'refs/tags/' -and $_ -notmatch '\\^{}' } | "
        "ForEach-Object { $_ -replace 'refs/tags/', '' } | "
        "Sort-Object -Descending | Select-Object -First 1 | "
        "ForEach-Object { "
            "$version = $_; "
            "$url = \"https://gitcode.com/2401_86702825/Quern/releases/download/$version/Quern-Linux%26Windows-amd64-LSDK.zip\"; "
            "Write-Host \"New Version: $version\"; "
            "Write-Host \"Download url: $url\"; "
            "Invoke-WebRequest -Uri $url -OutFile \"Quern-Linux&Windows-amd64-LSDK.zip\"; "
            "Expand-Archive -Path \"Quern-Linux&Windows-amd64-LSDK.zip\" -DestinationPath \".\" -Force; "
            "Remove-Item \"Quern-Linux&Windows-amd64-LSDK.zip\"; "
            "Set-Location \"Quern\"; "
            "Write-Host \"[Updater] Update Done.\" "
        "}";

    int wlen = MultiByteToWideChar(CP_UTF8, 0, psScript.c_str(), -1, NULL, 0);
    std::vector<wchar_t> wstr(wlen);
    MultiByteToWideChar(CP_UTF8, 0, psScript.c_str(), -1, wstr.data(), wlen);
    std::string utf16Script((const char*)wstr.data(), (wlen - 1) * sizeof(wchar_t));
    std::string encoded = Base64Encode(utf16Script);
    std::string cmd = "powershell.exe -ExecutionPolicy Bypass -NoProfile -EncodedCommand " + encoded;
    int result = system(cmd.c_str());
    if (result != 0) {
        std::cerr << "[Updater] Error: Command failed with exit code " << result << "\n";
    }

#elif __linux__
    std::cout << "[Updater] Detected Linux OS.\n";
    std::cout << "[Updater] Executing update command...\n";

    std::string ver = getLatestTag();
    if(ver.empty())
    {
        std::cerr << "[Updater] Failed fetch latest tag!\n";
        return;
    }
    std::string url = "https://gitcode.com/2401_86702825/Quern/releases/download/"
                      + ver
                      + "/Quern-Linux%26Windows-amd64-LSDK.zip";

    std::cout << "[DEBUG] Final Download URL: " << url << std::endl;

    int ret = runWget(url);
    if(ret != 0)
    {
        std::cerr << "[Updater] wget failed!\n";
        return;
    }
    std::string postCmd = "unzip -o 'Quern-Linux&Windows-amd64-LSDK.zip' -d . && rm 'Quern-Linux&Windows-amd64-LSDK.zip' && cd Quern";
    runShellCmd(postCmd);
#endif

    std::cout << "       Update Process Finished\n";
}