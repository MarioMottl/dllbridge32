use libloading::Library;
use std::env::args;
use std::ffi::CString;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;
use std::thread;

#[derive(Debug)]
enum SupportedType {
    Int,
    Float,
    Char,
    Void,
}

impl FromStr for SupportedType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "int" => Ok(SupportedType::Int),
            "float" => Ok(SupportedType::Float),
            "char" => Ok(SupportedType::Char),
            "void" => Ok(SupportedType::Void),
            _ => Err(format!("Unsupported type: {}", s)),
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
struct FunctionSignature {
    calling_convention: String, // e.g., "cdecl" or "stdcall"
    param_types: Vec<SupportedType>,
    return_type: SupportedType,
}

fn parse_signature(signature: &str) -> Result<FunctionSignature, String> {
    let parts: Vec<&str> = signature.split("->").collect();
    if parts.len() != 2 {
        return Err("Signature must contain '->'".into());
    }
    let params_with_conv = parts[0]; // e.g., "int,int(stdcall)"
    let ret_type_str = parts[1]; // e.g., "int"

    let mut calling_convention = "cdecl".to_string();
    let params_part = if let Some(start) = params_with_conv.find('(') {
        if let Some(end) = params_with_conv.find(')') {
            calling_convention = params_with_conv[start + 1..end].to_string();
            &params_with_conv[..start]
        } else {
            return Err("Malformed signature: missing closing parenthesis".into());
        }
    } else {
        params_with_conv
    };

    let param_types: Result<Vec<SupportedType>, String> = params_part
        .split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().parse())
        .collect();
    let param_types = param_types?;
    let return_type = ret_type_str.trim().parse()?;

    Ok(FunctionSignature {
        calling_convention,
        param_types,
        return_type,
    })
}

fn dynamic_invoke(func_ptr: *const std::ffi::c_void, args: &[&str]) -> Result<String, String> {
    use libffi::middle::{Arg, Cif, CodePtr, Type};

    let arg_types = vec![Type::i32(); args.len()];
    let cif = Cif::new(arg_types, Type::i32());

    let parsed_args: Result<Vec<i32>, _> = args.iter().map(|s| s.parse::<i32>()).collect();
    let parsed_args = parsed_args.map_err(|_| "Argument parsing error".to_string())?;

    let ffi_args: Vec<Arg> = parsed_args.iter().map(|a| Arg::new(a)).collect();
    let code_ptr = CodePtr::from_ptr(func_ptr);
    let result: i32 = unsafe { cif.call(code_ptr, &ffi_args) };

    Ok(result.to_string())
}

fn invoke_function(
    lib: &Library,
    name: &str,
    metadata: Option<&str>,
    args: &[&str],
) -> Result<String, String> {
    let func_name = CString::new(name).map_err(|_| "Invalid function name")?;
    unsafe {
        let symbol = lib
            .get::<*const ()>(func_name.as_bytes_with_nul())
            .map_err(|e| e.to_string())?;
        let func_ptr = *symbol as *const std::ffi::c_void;

        if let Some(return_str) = metadata {
            let signature = parse_signature(return_str)?;
            println!("Using metadata: {:?}", signature);
            dynamic_invoke(func_ptr, args)
        } else {
            Err("No signature string provided".into())
        }
    }
}

fn handle_client_command(stream: &mut TcpStream, lib: &Library, line: &str) -> () {
    let tokens: Vec<&str> = line.trim().split_whitespace().collect();
    if tokens.get(0) != Some(&"call") {
        stream
            .write_all(b"ERR Command must start with 'call'")
            .expect("Could not write to stream");

        return;
    }
    if tokens.len() < 2 {
        stream
            .write_all(b"ERR Missing function name")
            .expect("Could not write to stream");
        return;
    }

    let function_name = tokens[1];

    let mut metadata: Option<String> = None;
    let mut args_start = 2;

    if let Some(&sig_tok) = tokens.get(2) {
        if sig_tok.starts_with("sig:") {
            let mut sig = String::new();
            let mut end_idx = 2;
            for (i, &tok) in tokens.iter().enumerate().skip(2) {
                let piece = if i == 2 {
                    tok.trim_start_matches("sig:")
                } else {
                    tok
                };
                if !sig.is_empty() {
                    sig.push(' ');
                }
                sig.push_str(piece);
                if sig.contains("->") {
                    end_idx = i;
                    break;
                }
            }
            if !sig.contains("->") {
                stream
                    .write_all(b"ERR Malformed signature; no '->' found")
                    .expect("Could not write to stream");
                return;
            }
            metadata = Some(sig);
            args_start = end_idx + 1;
        }
    }

    let args = &tokens[args_start..];

    match invoke_function(lib, function_name, metadata.as_deref(), args) {
        Ok(res) => stream
            .write_all(format!("{res}").as_bytes())
            .expect("Could not write to stream"),
        Err(err) => stream
            .write_all(format!("ERR {}", err).as_bytes())
            .expect("Could not write to stream"),
    };
}

fn handle_client(mut stream: TcpStream, lib: Arc<Library>) {
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line) {
        if n == 0 {
            break;
        }
        handle_client_command(&mut stream, &lib, &line);
        line.clear();
    }
}

fn main() {
    let args: Vec<String> = args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <path_to_dll> [port]", args[0]);
        std::process::exit(1);
    }
    let dll_path = &args[1];
    let port = if args.len() >= 3 { &args[2] } else { "5000" };

    let lib = unsafe {
        Library::new(dll_path).unwrap_or_else(|e| {
            eprintln!("Failed to load DLL {}: {}", dll_path, e);
            std::process::exit(1);
        })
    };
    println!("Loaded DLL: {}", dll_path);

    let lib = Arc::new(lib);

    let listener_addr = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(&listener_addr).unwrap_or_else(|e| {
        eprintln!("Failed to bind to {}: {}", listener_addr, e);
        std::process::exit(1);
    });
    println!("DLL server listening on {}", listener_addr);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let lib_clone = Arc::clone(&lib);
                thread::spawn(move || {
                    handle_client(stream, lib_clone);
                });
            }
            Err(e) => eprintln!("Connection failed: {}", e),
        }
    }
}
