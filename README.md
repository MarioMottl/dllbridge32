# DLLBridge32

## DLLBridge32

is a lightweight compatibility layer that allows a 64-bit client to invoke functions from a 32-bit DLL. It does so by running a 32-bit Rust-based server that dynamically loads a specified DLL and exposes its functions over a simple TCP socket. This project is particularly useful for testing environments and hardware-in-the-loop (HIL) setups where legacy 32-bit libraries need to be used alongside 64-bit systems

## Features

Dynamic DLL Loading: Load any 32-bit DLL at runtime.
Function Enumeration & Invocation: Call exported functions by name. Functions can be invoked with default assumptions or with optional metadata to specify the signature.
Hybrid Invocation Mode: Support for both default assumptions (e.g., no parameters or two integers) and metadata-driven dynamic calls (using a simple signature format).
Lightweight IPC: Communicates over TCP (using localhost) to allow 64-bit clients to interact with the 32-bit server.
Minimal Dependencies: Built using Rust with minimal external crates for efficiency and ease of maintenance.
Usage

### Building the Server

Windows (Cross-Compilation from Linux)
Add the Windows target:

```bash
rustup target add i686-pc-windows-gnu
Ensure the mingw-w64 toolchain is installed:
```

```bash
sudo apt-get install gcc-mingw-w64-i686
Build the Windows 32-bit version:
```

```bash
cargo build --release --target i686-pc-windows-gnu
Alternatively, you can use a task runner (like cargo-make)
    with a Makefile.toml to build both targets with a single command:
```

```bash
cargo make build-all
Note: If you encounter errors regarding the libffi dependency, install the development package on Linux:
```

```bash
sudo apt-get install libffi-dev
Running the Server
```

Launch the server from a command prompt. For example, on Windows:

```bash
dll_server.exe sample_dll.dll 5000
This command loads sample_dll.dll and listens for incoming client connections on port 5000.
```

Sending Commands
Use Telnet or Netcat to connect to the server:

```bash
telnet 127.0.0.1 5000
```

Example Commands
Call a no-parameter function:

```bash
call helloworld
Expected response: 42
```

Call a function with two integers (default two-int signature):

```bash
call AddNumbers 5 7
Expected response: 12
```

Call a function using metadata (specifying calling convention):

```bash
call ComputeSumStdCall sig:int,int(stdcall)->int 8 9
Expected response: 17
```

## Sample DLL

A sample 32-bit DLL (testlib.dll) written in C is provided. It exports the following functions:

`helloworld`: No parameters; returns an integer.
`AddNumbers`: Takes two integers; returns their sum.
`ComputeSumStdCall`: Uses the stdcall convention; takes two integers; returns their sum.
Compile the sample DLL with a command such as:

```bash
cl /LD /W3 /MD sample_dll.c /Fe:sample_dll.dll
```

Ensure that you compile for 32-bit (using an appropriate x86 Developer Command Prompt).
