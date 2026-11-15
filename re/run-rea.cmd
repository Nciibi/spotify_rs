@echo off
set "PATH=C:\Users\rrr\Tools\node26\node-v26.13.0-win-x64;%PATH%"
set "JAVA_HOME=C:\Users\rrr\Tools\jdk21\jdk-21.0.13+9"
set "GHIDRA_INSTALL_DIR=C:\Users\rrr\Tools\ghidra\ghidra_12.1.4_PUBLIC"
set "REA_GHIDRA_STARTUP_TIMEOUT_MS=5400000"
set "REA=C:\Users\rrr\Tools\node26\node-v26.13.0-win-x64\rea.cmd"
%REA% %*
