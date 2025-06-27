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
}

impl FromStr for SupportedType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "int" => Ok(SupportedType::Int),
            "float" => Ok(SupportedType::Float),
            "char" => Ok(SupportedType::Char),
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

fn handle_client_command(stream: &mut TcpStream, lib: &Library, line: &str) {
    let tokens: Vec<&str> = line.trim().split_whitespace().collect();
    if tokens.len() < 2 {
        writeln!(stream, "ERR Invalid command").ok();
        return;
    }
    let function_name = tokens[1];
    let mut metadata = None;
    let mut args_start_index = 2;

    if tokens.len() > 2 && tokens[2].starts_with("sig:") {
        let sig = tokens[2].trim_start_matches("sig:");
        metadata = Some(sig);
        args_start_index = 3;
    }
    let args = &tokens[args_start_index..];

    match invoke_function(lib, function_name, metadata, args) {
        Ok(result) => {
            writeln!(stream, "{}", result).ok();
        }
        Err(err_msg) => {
            writeln!(stream, "ERR {}", err_msg).ok();
        }
    }
}

fn handle_client(mut stream: TcpStream, lib: Arc<Library>) {
    println!("Client connected: {}", stream.peer_addr().unwrap());
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut line = String::new();
    while let Ok(bytes_read) = reader.read_line(&mut line) {
        if bytes_read == 0 {
            break;
        }
        if line.trim().starts_with("call") {
            handle_client_command(&mut stream, &lib, &line);
        } else {
            writeln!(stream, "ERR Unknown command").ok();
        }
        line.clear();
    }
    println!("Client disconnected.");
}

fn main() {
    // Usage: dll_server <path_to_dll> [port]
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
