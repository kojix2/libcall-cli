use crate::callback::{clear_global_callback, set_global_callback, LuaCallback};
use crate::parser::Argument;
use crate::types::{Type, Value};
use anyhow::{anyhow, Result};
use std::ffi::{c_void, CString};

#[derive(Debug)]
pub struct CallResult {
    pub return_value: Option<Value>,
    pub output_values: Vec<(usize, Value)>,
}

pub fn execute_call(
    func_ptr: *mut c_void,
    args: &mut [Argument],
    return_type: Type,
) -> Result<CallResult> {
    let mut arg_ptrs: Vec<*mut c_void> = Vec::new();
    let mut arg_storage: Vec<Box<dyn std::any::Any>> = Vec::new();
    let mut cstrings: Vec<CString> = Vec::new();
    let mut arg_is_ptr_for_call: Vec<bool> = Vec::new();
    let mut callback_storage: Vec<Box<LuaCallback>> = Vec::new();

    for arg in args.iter_mut() {
        match &mut arg.value {
            Value::I8(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(
                    arg_storage.last().unwrap().downcast_ref::<i8>().unwrap() as *const i8
                        as *mut c_void,
                );
                arg_is_ptr_for_call.push(false);
            }
            Value::U8(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(
                    arg_storage.last().unwrap().downcast_ref::<u8>().unwrap() as *const u8
                        as *mut c_void,
                );
                arg_is_ptr_for_call.push(false);
            }
            Value::I16(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<i16>().unwrap()
                    as *const i16 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::U16(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<u16>().unwrap()
                    as *const u16 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::I32(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<i32>().unwrap()
                    as *const i32 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::U32(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<u32>().unwrap()
                    as *const u32 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::I64(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<i64>().unwrap()
                    as *const i64 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::U64(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<u64>().unwrap()
                    as *const u64 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::Isize(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<isize>().unwrap()
                    as *const isize as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::Usize(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<usize>().unwrap()
                    as *const usize as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::F32(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<f32>().unwrap()
                    as *const f32 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::F64(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(arg_storage.last().unwrap().downcast_ref::<f64>().unwrap()
                    as *const f64 as *mut c_void);
                arg_is_ptr_for_call.push(false);
            }
            Value::Ptr(v) => {
                arg_storage.push(Box::new(*v));
                arg_ptrs.push(
                    arg_storage
                        .last()
                        .unwrap()
                        .downcast_ref::<*mut c_void>()
                        .unwrap() as *const *mut c_void as *mut c_void,
                );
                arg_is_ptr_for_call.push(false);
            }
            Value::CStr(s) => {
                let cstr = CString::new(s.as_str())?;
                let ptr = cstr.as_ptr() as *mut c_void;
                cstrings.push(cstr);
                arg_ptrs.push(ptr);
                arg_is_ptr_for_call.push(false);
            }
            Value::Array {
                elem_type, values, ..
            } => {
                let ptr = create_array_storage(*elem_type, values)?;
                arg_storage.push(Box::new(ptr));
                arg_ptrs.push(ptr);
                arg_is_ptr_for_call.push(true);
            }
            Value::Callback { signature, body } => {
                let callback = Box::new(LuaCallback::new(signature.clone(), body.clone())?);
                let callback_ptr = callback.as_ref() as *const LuaCallback;

                unsafe {
                    set_global_callback(callback_ptr);
                }

                let wrapper_ptr = callback.get_c_wrapper();
                callback_storage.push(callback);

                arg_ptrs.push(wrapper_ptr);
                arg_is_ptr_for_call.push(true);
            }
        }
    }

    let return_value =
        unsafe { call_function_dynamic(func_ptr, &arg_ptrs, &arg_is_ptr_for_call, return_type)? };

    unsafe {
        clear_global_callback();
    }

    let mut output_values = Vec::new();
    for (idx, arg) in args.iter().enumerate() {
        if arg.is_output {
            if let Value::Array {
                elem_type, values, ..
            } = &arg.value
            {
                let ptr = arg_ptrs[idx];
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

fn create_array_storage(elem_type: Type, values: &[Value]) -> Result<*mut c_void> {
    let size = elem_type.size();
    let total_size = size * values.len();

    let layout = std::alloc::Layout::from_size_align(total_size, size)
        .map_err(|e| anyhow!("Failed to create layout: {}", e))?;

    let ptr = unsafe { std::alloc::alloc(layout) as *mut c_void };

    if ptr.is_null() {
        return Err(anyhow!("Failed to allocate memory for array"));
    }

    for (i, value) in values.iter().enumerate() {
        let offset = i * size;
        unsafe {
            let dest = (ptr as *mut u8).add(offset);
            write_value_to_ptr(dest as *mut c_void, value)?;
        }
    }

    Ok(ptr)
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

    Ok(Value::Array {
        elem_type,
        values,
        is_output: true,
    })
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
    args: &[*mut c_void],
    arg_is_ptr: &[bool],
    return_type: Type,
) -> Result<Option<Value>> {
    match return_type {
        Type::Void => {
            type VoidFunc0 = unsafe extern "C" fn();
            type VoidFunc4 = unsafe extern "C" fn(*mut c_void, usize, usize, *mut c_void);

            match args.len() {
                0 => {
                    let func = std::mem::transmute::<*mut c_void, VoidFunc0>(func_ptr);
                    func();
                }
                4 => {
                    let func = std::mem::transmute::<*mut c_void, VoidFunc4>(func_ptr);
                    func(
                        args[0],
                        *(args[1] as *const usize),
                        *(args[2] as *const usize),
                        args[3],
                    );
                }
                _ => return Err(anyhow!("Unsupported argument count for void return")),
            }
            Ok(None)
        }
        Type::I32 => {
            type I32Func0 = unsafe extern "C" fn() -> i32;
            type I32Func1 = unsafe extern "C" fn(*mut c_void) -> i32;
            type I32Func2 = unsafe extern "C" fn(*mut c_void, *mut c_void) -> i32;

            let result = match args.len() {
                0 => {
                    let func = std::mem::transmute::<*mut c_void, I32Func0>(func_ptr);
                    func()
                }
                1 => {
                    let func = std::mem::transmute::<*mut c_void, I32Func1>(func_ptr);
                    func(args[0])
                }
                2 => {
                    let func = std::mem::transmute::<*mut c_void, I32Func2>(func_ptr);
                    func(args[0], args[1])
                }
                _ => return Err(anyhow!("Too many arguments")),
            };
            Ok(Some(Value::I32(result)))
        }
        Type::F64 => {
            type F64Func1Val = unsafe extern "C" fn(f64) -> f64;
            type F64Func2ValVal = unsafe extern "C" fn(f64, f64) -> f64;
            type F64Func2ValPtr = unsafe extern "C" fn(f64, *mut f64) -> f64;

            let result = match args.len() {
                1 => {
                    let func = std::mem::transmute::<*mut c_void, F64Func1Val>(func_ptr);
                    let val = *(args[0] as *const f64);
                    func(val)
                }
                2 => {
                    let val1 = *(args[0] as *const f64);

                    if arg_is_ptr[1] {
                        let func = std::mem::transmute::<*mut c_void, F64Func2ValPtr>(func_ptr);
                        func(val1, args[1] as *mut f64)
                    } else {
                        let func = std::mem::transmute::<*mut c_void, F64Func2ValVal>(func_ptr);
                        let val2 = *(args[1] as *const f64);
                        func(val1, val2)
                    }
                }
                _ => return Err(anyhow!("Unsupported argument count for f64 return")),
            };
            Ok(Some(Value::F64(result)))
        }
        Type::Usize => {
            type UsizeFunc1 = unsafe extern "C" fn(*mut c_void) -> usize;

            let result = match args.len() {
                1 => {
                    let func = std::mem::transmute::<*mut c_void, UsizeFunc1>(func_ptr);
                    func(args[0])
                }
                _ => return Err(anyhow!("Unsupported argument count for usize return")),
            };
            Ok(Some(Value::Usize(result)))
        }
        Type::CStr => {
            type CStrFunc1 = unsafe extern "C" fn(*mut c_void) -> *const std::ffi::c_char;

            let result = match args.len() {
                1 => {
                    let func = std::mem::transmute::<*mut c_void, CStrFunc1>(func_ptr);
                    let ptr = func(args[0]);
                    if ptr.is_null() {
                        String::new()
                    } else {
                        std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned()
                    }
                }
                _ => return Err(anyhow!("Unsupported argument count for cstr return")),
            };
            Ok(Some(Value::CStr(result)))
        }
        _ => Err(anyhow!("Unsupported return type: {}", return_type)),
    }
}
