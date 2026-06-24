use crate::callback::{clear_global_callback, set_global_callback, LuaCallback};
use crate::parser::Argument;
use crate::types::{Type, Value};
use anyhow::{anyhow, Result};
use std::ffi::{c_void, CString};
use std::ptr::NonNull;

unsafe extern "C" {
    fn fflush(stream: *mut c_void) -> i32;
}

#[derive(Debug)]
pub struct CallResult {
    pub return_value: Option<Value>,
    pub output_values: Vec<(usize, Value)>,
}

#[derive(Debug, Clone, Copy)]
enum CallArg {
    I8(i8),
    U8(u8),
    I16(i16),
    U16(u16),
    I32(i32),
    U32(u32),
    I64(i64),
    U64(u64),
    Isize(isize),
    Usize(usize),
    F32(f32),
    F64(f64),
    Ptr(*mut c_void),
}

impl CallArg {
    fn pointer(self) -> Result<*mut c_void> {
        match self {
            CallArg::Ptr(ptr) => Ok(ptr),
            other => Err(anyhow!(
                "Expected pointer argument, got {}",
                other.type_name()
            )),
        }
    }

    fn type_name(self) -> &'static str {
        match self {
            CallArg::I8(_) => "i8",
            CallArg::U8(_) => "u8",
            CallArg::I16(_) => "i16",
            CallArg::U16(_) => "u16",
            CallArg::I32(_) => "i32",
            CallArg::U32(_) => "u32",
            CallArg::I64(_) => "i64",
            CallArg::U64(_) => "u64",
            CallArg::Isize(_) => "isize",
            CallArg::Usize(_) => "usize",
            CallArg::F32(_) => "f32",
            CallArg::F64(_) => "f64",
            CallArg::Ptr(_) => "ptr",
        }
    }
}

struct ArrayStorage {
    ptr: NonNull<c_void>,
    layout: std::alloc::Layout,
}

impl ArrayStorage {
    fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

impl Drop for ArrayStorage {
    fn drop(&mut self) {
        unsafe {
            std::alloc::dealloc(self.ptr.as_ptr() as *mut u8, self.layout);
        }
    }
}

struct CallbackGuard {
    active: bool,
}

impl CallbackGuard {
    fn new() -> Self {
        Self { active: true }
    }
}

impl Drop for CallbackGuard {
    fn drop(&mut self) {
        if self.active {
            clear_global_callback();
        }
    }
}

pub fn execute_call(
    func_ptr: *mut c_void,
    args: &mut [Argument],
    return_type: Type,
) -> Result<CallResult> {
    let mut call_args: Vec<CallArg> = Vec::new();
    let mut array_storage: Vec<ArrayStorage> = Vec::new();
    let mut cstrings: Vec<CString> = Vec::new();
    let mut callback_storage: Vec<Box<LuaCallback>> = Vec::new();
    let mut callback_guard = None;

    for arg in args.iter_mut() {
        match &mut arg.value {
            Value::I8(v) => {
                call_args.push(CallArg::I8(*v));
            }
            Value::U8(v) => {
                call_args.push(CallArg::U8(*v));
            }
            Value::I16(v) => {
                call_args.push(CallArg::I16(*v));
            }
            Value::U16(v) => {
                call_args.push(CallArg::U16(*v));
            }
            Value::I32(v) => {
                call_args.push(CallArg::I32(*v));
            }
            Value::U32(v) => {
                call_args.push(CallArg::U32(*v));
            }
            Value::I64(v) => {
                call_args.push(CallArg::I64(*v));
            }
            Value::U64(v) => {
                call_args.push(CallArg::U64(*v));
            }
            Value::Isize(v) => {
                call_args.push(CallArg::Isize(*v));
            }
            Value::Usize(v) => {
                call_args.push(CallArg::Usize(*v));
            }
            Value::F32(v) => {
                call_args.push(CallArg::F32(*v));
            }
            Value::F64(v) => {
                call_args.push(CallArg::F64(*v));
            }
            Value::Ptr(v) => {
                call_args.push(CallArg::Ptr(*v));
            }
            Value::CStr(s) => {
                let cstr = CString::new(s.as_str())?;
                let ptr = cstr.as_ptr() as *mut c_void;
                cstrings.push(cstr);
                call_args.push(CallArg::Ptr(ptr));
            }
            Value::Array {
                elem_type, values, ..
            } => {
                let storage = create_array_storage(*elem_type, values)?;
                call_args.push(CallArg::Ptr(storage.as_ptr()));
                array_storage.push(storage);
            }
            Value::Callback { signature, body } => {
                if !callback_storage.is_empty() {
                    return Err(anyhow!(
                        "Only one callback argument is currently supported per call"
                    ));
                }

                let callback = Box::new(LuaCallback::new(signature.clone(), body.clone())?);
                let callback_ptr = callback.as_ref() as *const LuaCallback;
                let wrapper_ptr = callback.get_c_wrapper()?;

                set_global_callback(callback_ptr);
                callback_guard = Some(CallbackGuard::new());

                callback_storage.push(callback);

                call_args.push(CallArg::Ptr(wrapper_ptr));
            }
        }
    }

    let return_value = unsafe { call_function_dynamic(func_ptr, &call_args, return_type)? };
    flush_c_stdio();
    drop(callback_guard.take());

    let mut output_values = Vec::new();
    for (idx, arg) in args.iter().enumerate() {
        if arg.is_output {
            if let Value::Array {
                elem_type, values, ..
            } = &arg.value
            {
                let ptr = call_args[idx].pointer()?;
                let output_array = read_array_from_ptr(ptr, *elem_type, values.len())?;
                output_values.push((idx, output_array));
            }
        }
    }

    Ok(CallResult {
        return_value,
        output_values,
    })
}

fn flush_c_stdio() {
    unsafe {
        fflush(std::ptr::null_mut());
    }
}

fn create_array_storage(elem_type: Type, values: &[Value]) -> Result<ArrayStorage> {
    let size = elem_type.size();
    let total_size = size * values.len();

    let layout = std::alloc::Layout::from_size_align(total_size, size)
        .map_err(|e| anyhow!("Failed to create layout: {}", e))?;

    let ptr = NonNull::new(unsafe { std::alloc::alloc(layout) as *mut c_void })
        .ok_or_else(|| anyhow!("Failed to allocate memory for array"))?;

    let storage = ArrayStorage { ptr, layout };

    for (i, value) in values.iter().enumerate() {
        let offset = i * size;
        unsafe {
            let dest = (storage.as_ptr() as *mut u8).add(offset);
            write_value_to_ptr(dest as *mut c_void, value)?;
        }
    }

    Ok(storage)
}

unsafe fn write_value_to_ptr(ptr: *mut c_void, value: &Value) -> Result<()> {
    match value {
        Value::I8(v) => *(ptr as *mut i8) = *v,
        Value::U8(v) => *(ptr as *mut u8) = *v,
        Value::I16(v) => *(ptr as *mut i16) = *v,
        Value::U16(v) => *(ptr as *mut u16) = *v,
        Value::I32(v) => *(ptr as *mut i32) = *v,
        Value::U32(v) => *(ptr as *mut u32) = *v,
        Value::I64(v) => *(ptr as *mut i64) = *v,
        Value::U64(v) => *(ptr as *mut u64) = *v,
        Value::Isize(v) => *(ptr as *mut isize) = *v,
        Value::Usize(v) => *(ptr as *mut usize) = *v,
        Value::F32(v) => *(ptr as *mut f32) = *v,
        Value::F64(v) => *(ptr as *mut f64) = *v,
        Value::Ptr(v) => *(ptr as *mut *mut c_void) = *v,
        _ => return Err(anyhow!("Unsupported value type for array element")),
    }
    Ok(())
}

fn read_array_from_ptr(ptr: *mut c_void, elem_type: Type, count: usize) -> Result<Value> {
    let size = elem_type.size();
    let mut values = Vec::new();

    for i in 0..count {
        let offset = i * size;
        let elem_ptr = unsafe { (ptr as *mut u8).add(offset) as *mut c_void };
        let value = unsafe { read_value_from_ptr(elem_ptr, elem_type)? };
        values.push(value);
    }

    Ok(Value::Array { elem_type, values })
}

unsafe fn read_value_from_ptr(ptr: *mut c_void, ty: Type) -> Result<Value> {
    match ty {
        Type::I8 => Ok(Value::I8(*(ptr as *const i8))),
        Type::U8 => Ok(Value::U8(*(ptr as *const u8))),
        Type::I16 => Ok(Value::I16(*(ptr as *const i16))),
        Type::U16 => Ok(Value::U16(*(ptr as *const u16))),
        Type::I32 => Ok(Value::I32(*(ptr as *const i32))),
        Type::U32 => Ok(Value::U32(*(ptr as *const u32))),
        Type::I64 => Ok(Value::I64(*(ptr as *const i64))),
        Type::U64 => Ok(Value::U64(*(ptr as *const u64))),
        Type::Isize => Ok(Value::Isize(*(ptr as *const isize))),
        Type::Usize => Ok(Value::Usize(*(ptr as *const usize))),
        Type::F32 => Ok(Value::F32(*(ptr as *const f32))),
        Type::F64 => Ok(Value::F64(*(ptr as *const f64))),
        Type::Ptr => Ok(Value::Ptr(*(ptr as *const *mut c_void))),
        _ => Err(anyhow!("Unsupported type for reading from pointer")),
    }
}

unsafe fn call_function_dynamic(
    func_ptr: *mut c_void,
    args: &[CallArg],
    return_type: Type,
) -> Result<Option<Value>> {
    macro_rules! call_with_return {
        ($ret:ty) => {{
            match args {
                [] => {
                    type Func = unsafe extern "C" fn() -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)()
                }
                [CallArg::I8(a)] => {
                    type Func = unsafe extern "C" fn(i8) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U8(a)] => {
                    type Func = unsafe extern "C" fn(u8) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::I16(a)] => {
                    type Func = unsafe extern "C" fn(i16) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U16(a)] => {
                    type Func = unsafe extern "C" fn(u16) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::I32(a)] => {
                    type Func = unsafe extern "C" fn(i32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U32(a)] => {
                    type Func = unsafe extern "C" fn(u32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::I64(a)] => {
                    type Func = unsafe extern "C" fn(i64) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U64(a)] => {
                    type Func = unsafe extern "C" fn(u64) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Isize(a)] => {
                    type Func = unsafe extern "C" fn(isize) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Usize(a)] => {
                    type Func = unsafe extern "C" fn(usize) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::F32(a)] => {
                    type Func = unsafe extern "C" fn(f32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::F64(a)] => {
                    type Func = unsafe extern "C" fn(f64) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Ptr(a)] => {
                    type Func = unsafe extern "C" fn(*mut c_void) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Ptr(a), CallArg::Ptr(b)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, *mut c_void) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::I32(a), CallArg::I32(b)] => {
                    type Func = unsafe extern "C" fn(i32, i32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::F32(a), CallArg::F32(b)] => {
                    type Func = unsafe extern "C" fn(f32, f32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::F64(a), CallArg::F64(b)] => {
                    type Func = unsafe extern "C" fn(f64, f64) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::F64(a), CallArg::Ptr(b)] => {
                    type Func = unsafe extern "C" fn(f64, *mut f64) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b as *mut f64)
                }
                [CallArg::Ptr(a), CallArg::Usize(b)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, usize) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::Ptr(a), CallArg::I32(b)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, i32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::Ptr(a), CallArg::Usize(b), CallArg::U32(c)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, usize, u32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b, *c)
                }
                [CallArg::Ptr(a), CallArg::Ptr(b), CallArg::I32(c)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, *mut c_void, i32) -> $ret;
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b, *c)
                }
                _ => return Err(unsupported_signature(return_type, args)),
            }
        }};
    }

    macro_rules! call_void {
        () => {{
            match args {
                [] => {
                    type Func = unsafe extern "C" fn();
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)()
                }
                [CallArg::I8(a)] => {
                    type Func = unsafe extern "C" fn(i8);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U8(a)] => {
                    type Func = unsafe extern "C" fn(u8);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::I16(a)] => {
                    type Func = unsafe extern "C" fn(i16);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U16(a)] => {
                    type Func = unsafe extern "C" fn(u16);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::I32(a)] => {
                    type Func = unsafe extern "C" fn(i32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U32(a)] => {
                    type Func = unsafe extern "C" fn(u32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::I64(a)] => {
                    type Func = unsafe extern "C" fn(i64);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::U64(a)] => {
                    type Func = unsafe extern "C" fn(u64);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Isize(a)] => {
                    type Func = unsafe extern "C" fn(isize);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Usize(a)] => {
                    type Func = unsafe extern "C" fn(usize);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::F32(a)] => {
                    type Func = unsafe extern "C" fn(f32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::F64(a)] => {
                    type Func = unsafe extern "C" fn(f64);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Ptr(a)] => {
                    type Func = unsafe extern "C" fn(*mut c_void);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a)
                }
                [CallArg::Ptr(a), CallArg::Ptr(b)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, *mut c_void);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::I32(a), CallArg::I32(b)] => {
                    type Func = unsafe extern "C" fn(i32, i32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::F32(a), CallArg::F32(b)] => {
                    type Func = unsafe extern "C" fn(f32, f32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::F64(a), CallArg::F64(b)] => {
                    type Func = unsafe extern "C" fn(f64, f64);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::F64(a), CallArg::Ptr(b)] => {
                    type Func = unsafe extern "C" fn(f64, *mut f64);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b as *mut f64)
                }
                [CallArg::Ptr(a), CallArg::Usize(b)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, usize);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::Ptr(a), CallArg::I32(b)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, i32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b)
                }
                [CallArg::Ptr(a), CallArg::Usize(b), CallArg::U32(c)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, usize, u32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b, *c)
                }
                [CallArg::Ptr(a), CallArg::Ptr(b), CallArg::I32(c)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, *mut c_void, i32);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b, *c)
                }
                [CallArg::Ptr(a), CallArg::Usize(b), CallArg::Usize(c), CallArg::Ptr(d)] => {
                    type Func = unsafe extern "C" fn(*mut c_void, usize, usize, *mut c_void);
                    std::mem::transmute::<*mut c_void, Func>(func_ptr)(*a, *b, *c, *d)
                }
                _ => return Err(unsupported_signature(return_type, args)),
            }
        }};
    }

    match return_type {
        Type::Void => {
            call_void!();
            Ok(None)
        }
        Type::I8 => Ok(Some(Value::I8(call_with_return!(i8)))),
        Type::U8 => Ok(Some(Value::U8(call_with_return!(u8)))),
        Type::I16 => Ok(Some(Value::I16(call_with_return!(i16)))),
        Type::U16 => Ok(Some(Value::U16(call_with_return!(u16)))),
        Type::I32 => Ok(Some(Value::I32(call_with_return!(i32)))),
        Type::U32 => Ok(Some(Value::U32(call_with_return!(u32)))),
        Type::I64 => Ok(Some(Value::I64(call_with_return!(i64)))),
        Type::U64 => Ok(Some(Value::U64(call_with_return!(u64)))),
        Type::Isize => Ok(Some(Value::Isize(call_with_return!(isize)))),
        Type::Usize => Ok(Some(Value::Usize(call_with_return!(usize)))),
        Type::F32 => Ok(Some(Value::F32(call_with_return!(f32)))),
        Type::F64 => Ok(Some(Value::F64(call_with_return!(f64)))),
        Type::Ptr => Ok(Some(Value::Ptr(call_with_return!(*mut c_void)))),
        Type::CStr => {
            type CStrFunc1 = unsafe extern "C" fn(*mut c_void) -> *const std::ffi::c_char;

            let result = match args {
                [CallArg::Ptr(a)] => {
                    let func = std::mem::transmute::<*mut c_void, CStrFunc1>(func_ptr);
                    let ptr = func(*a);
                    if ptr.is_null() {
                        String::new()
                    } else {
                        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
                    }
                }
                _ => return Err(unsupported_signature(return_type, args)),
            };
            Ok(Some(Value::CStr(result)))
        }
        Type::Callback => Err(anyhow!("Unsupported return type: {}", return_type)),
    }
}

fn unsupported_signature(return_type: Type, args: &[CallArg]) -> anyhow::Error {
    let arg_types = args
        .iter()
        .map(|arg| arg.type_name())
        .collect::<Vec<_>>()
        .join(", ");
    anyhow!("Unsupported signature: ({}) -> {}", arg_types, return_type)
}
