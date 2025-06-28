#if defined(__GNUC__) && __GNUC__ >= 4
  #define EXPORT __attribute__((visibility("default")))
#else
  #define EXPORT
#endif

EXPORT int helloworld(void) {
    return 42;
}

EXPORT int AddNumbers(int a, int b) {
    return a + b;
}

EXPORT int ComputeSumStdCall(int a, int b) {
    return a + b;
}
