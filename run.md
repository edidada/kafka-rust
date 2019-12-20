# run


[](https://www.jianshu.com/p/2d4c330e4b25)

[](https://rust.cc/article?id=f17c50d3-1a76-4e92-b2a7-7bb9fa7bb622)


https://github.com/sfackler/rust-openssl/issues/1062

1.安装vcpkg
2.安装cmake
3.用vcpkg安装openssl, 执行vcpkg install openssl:x64-windows-static
4.设置环境变量，让OPENSSL_DIR=%Vcpkg目录%\installed\x64-windows-static
如果 openssl = { version = "0.10", features = ["vendored"] } 还需要安装ActivePerl
Ryan-Git 2019-12-05 18:04
交叉编译很麻烦，可以试试 rusttls

```shell

No host reachable
error: process didn't exit successfully: `target\debug\examples\console-producer.exe` (exit code: 1)

```


```shell

C:/Users/edidada/.cargo/bin/cargo.exe test --no-run --package kafka --test test_kafka integration::consumer_producer::producer::test_producer_send -- --exact
   Compiling openssl-sys v0.9.53
error: failed to run custom build command for `openssl-sys v0.9.53`
process didn't exit successfully: `D:\git\github\kafka-rust\target\debug\build\openssl-sys-ace0a4e9996c2446\build-script-main` (exit code: 101)
--- stdout
cargo:rustc-cfg=const_fn
cargo:rerun-if-env-changed=X86_64_PC_WINDOWS_MSVC_OPENSSL_LIB_DIR
X86_64_PC_WINDOWS_MSVC_OPENSSL_LIB_DIR unset
cargo:rerun-if-env-changed=OPENSSL_LIB_DIR
OPENSSL_LIB_DIR unset
cargo:rerun-if-env-changed=X86_64_PC_WINDOWS_MSVC_OPENSSL_INCLUDE_DIR
X86_64_PC_WINDOWS_MSVC_OPENSSL_INCLUDE_DIR unset
cargo:rerun-if-env-changed=OPENSSL_INCLUDE_DIR
OPENSSL_INCLUDE_DIR unset
cargo:rerun-if-env-changed=X86_64_PC_WINDOWS_MSVC_OPENSSL_DIR
X86_64_PC_WINDOWS_MSVC_OPENSSL_DIR unset
cargo:rerun-if-env-changed=OPENSSL_DIR
OPENSSL_DIR unset
note: vcpkg did not find openssl as libcrypto and libssl: Aborted because VCPKGRS_DYNAMIC is not set
note: vcpkg did not find openssl as ssleay32 and libeay32: Aborted because VCPKGRS_DYNAMIC is not set

--- stderr
thread 'main' panicked at '

Could not find directory of OpenSSL installation, and this `-sys` crate cannot
proceed without this knowledge. If OpenSSL is installed and this crate had
trouble finding it,  you can set the `OPENSSL_DIR` environment variable for the
compilation process.

Make sure you also have the development packages of openssl installed.
For example, `libssl-dev` on Ubuntu or `openssl-devel` on Fedora.

If you're in a situation where you think the directory *should* be found
automatically, please open a bug at https://github.com/sfackler/rust-openssl
and include information about your system as well as this message.

$HOST = x86_64-pc-windows-msvc
$TARGET = x86_64-pc-windows-msvc
openssl-sys = 0.9.53


It looks like you're compiling for MSVC but we couldn't detect an OpenSSL
installation. If there isn't one installed then you can try the rust-openssl
README for more information about how to download precompiled binaries of
OpenSSL:

https://github.com/sfackler/rust-openssl#windows

', C:\Users\edidada\.cargo\registry\src\mirrors.ustc.edu.cn-61ef6e0cd06fb9b8\openssl-sys-0.9.53\build\find_normal.rs:150:5
stack backtrace:
   0: std::sys::windows::backtrace::set_frames
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\sys\windows\backtrace\mod.rs:94
   1: std::sys::windows::backtrace::unwind_backtrace
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\sys\windows\backtrace\mod.rs:81
   2: std::sys_common::backtrace::_print
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\sys_common\backtrace.rs:70
   3: std::sys_common::backtrace::print
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\sys_common\backtrace.rs:58
   4: std::panicking::default_hook::{{closure}}
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\panicking.rs:200
   5: std::panicking::default_hook
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\panicking.rs:215
   6: std::panicking::rust_panic_with_hook
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\panicking.rs:478
   7: std::panicking::begin_panic<alloc::string::String>
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\src\libstd\panicking.rs:412
   8: build_script_main::find::find_openssl_dir
             at .\build\find_normal.rs:150
   9: build_script_main::find::get_openssl::{{closure}}
             at .\build\find_normal.rs:13
  10: core::option::Option<std::ffi::os_str::OsString>::unwrap_or_else<std::ffi::os_str::OsString,closure>
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\src\libcore\option.rs:386
  11: build_script_main::find::get_openssl
             at .\build\find_normal.rs:13
  12: build_script_main::main
             at .\build\main.rs:49
  13: std::rt::lang_start::{{closure}}<()>
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\src\libstd\rt.rs:64
  14: std::rt::lang_start_internal::{{closure}}
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\rt.rs:49
  15: std::panicking::try::do_call<closure,i32>
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\panicking.rs:297
  16: panic_unwind::__rust_maybe_catch_panic
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libpanic_unwind\lib.rs:87
  17: std::panicking::try
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\panicking.rs:276
  18: std::panic::catch_unwind
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\panic.rs:388
  19: std::rt::lang_start_internal
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\/src\libstd\rt.rs:48
  20: std::rt::lang_start<()>
             at /rustc/91856ed52c58aa5ba66a015354d1cc69e9779bdf\src\libstd\rt.rs:64
  21: main
  22: invoke_main
             at d:\agent\_work\3\s\src\vctools\crt\vcstartup\src\startup\exe_common.inl:78
  23: __scrt_common_main_seh
             at d:\agent\_work\3\s\src\vctools\crt\vcstartup\src\startup\exe_common.inl:288
  24: BaseThreadInitThunk
  25: RtlUserThreadStart
```
