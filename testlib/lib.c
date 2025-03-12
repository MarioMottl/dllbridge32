#include <windows.h>

// Exported function with cdecl (default) convention
__declspec(dllexport) int helloworld(void) {
  // For example, return a fixed value.
  return 42;
}

// Exported function with cdecl convention: adds two integers
__declspec(dllexport) int AddNumbers(int a, int b) { return a + b; }

// Exported function with stdcall convention: adds two integers
__declspec(dllexport) int __stdcall ComputeSumStdCall(int a, int b) {
  return a + b;
}

// Standard DLL entry point.
BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpReserved) {
  return TRUE;
}
