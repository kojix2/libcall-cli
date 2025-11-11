use crate::ffi::CallResult;
use crate::types::Value;

pub fn print_result_human(result: &CallResult, _function: &str) {
    if let Some(ref ret_val) = result.return_value {
        print_value(ret_val);
        println!();
    }

    if !result.output_values.is_empty() {
        println!("Outputs:");
        for (idx, val) in &result.output_values {
            print!("  [{}] ", idx);
            print_value_with_type(val);
            println!();
        }
    }
}

fn print_value(value: &Value) {
    match value {
        Value::I8(v) => print!("{}", v),
        Value::U8(v) => print!("{}", v),
        Value::I16(v) => print!("{}", v),
        Value::U16(v) => print!("{}", v),
        Value::I32(v) => print!("{}", v),
        Value::U32(v) => print!("{}", v),
        Value::I64(v) => print!("{}", v),
        Value::U64(v) => print!("{}", v),
        Value::Isize(v) => print!("{}", v),
        Value::Usize(v) => print!("{}", v),
        Value::F32(v) => print!("{}", v),
        Value::F64(v) => print!("{}", v),
        Value::Ptr(v) => print!("{:p}", v),
        Value::CStr(s) => print!("{}", s),
        Value::Array { .. } => print_array(value),
        Value::Callback { .. } => print!("<callback>"),
    }
}

fn print_value_with_type(value: &Value) {
    match value {
        Value::Array {
            elem_type, values, ..
        } => {
            if values.len() == 1 {
                print!("{} = ", elem_type);
                print_value(&values[0]);
            } else {
                print!("{}{} = ", values.len(), elem_type);
                print_array(value);
            }
        }
        _ => {
            print!("{} = ", value.get_type());
            print_value(value);
        }
    }
}

fn print_array(value: &Value) {
    if let Value::Array {
        elem_type: _,
        values,
        ..
    } = value
    {
        print!("[");
        for (i, val) in values.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            if i >= 10 {
                print!("... ({} more)", values.len() - 10);
                break;
            }
            match val {
                Value::U8(v) => print!("0x{:02x}", v),
                _ => print_value(val),
            }
        }
        print!("]");
    }
}

pub fn print_result_json(
    result: &CallResult,
    library: &str,
    function: &str,
    args: &[crate::parser::Argument],
) {
    use serde_json::json;

    let args_json: Vec<_> = args
        .iter()
        .map(|arg| {
            json!({
                "type": arg.value.get_type().to_string(),
                "value": value_to_json(&arg.value),
            })
        })
        .collect();

    let return_json = result.return_value.as_ref().map(|v| {
        json!({
            "type": v.get_type().to_string(),
            "value": value_to_json(v),
        })
    });

    let outputs_json: Vec<_> = result
        .output_values
        .iter()
        .map(|(idx, val)| {
            json!({
                "index": idx,
                "type": val.get_type().to_string(),
                "value": value_to_json(val),
            })
        })
        .collect();

    let output = json!({
        "library": library,
        "function": function,
        "args": args_json,
        "return": return_json,
        "outputs": outputs_json,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}

pub fn print_result_yaml(
    result: &CallResult,
    library: &str,
    function: &str,
    args: &[crate::parser::Argument],
) {
    use serde_json::json;

    let args_yaml: Vec<_> = args
        .iter()
        .map(|arg| {
            json!({
                "type": arg.value.get_type().to_string(),
                "value": value_to_json(&arg.value),
            })
        })
        .collect();

    let return_yaml = result.return_value.as_ref().map(|v| {
        json!({
            "type": v.get_type().to_string(),
            "value": value_to_json(v),
        })
    });

    let outputs_yaml: Vec<_> = result
        .output_values
        .iter()
        .map(|(idx, val)| {
            json!({
                "index": idx,
                "type": val.get_type().to_string(),
                "value": value_to_json(val),
            })
        })
        .collect();

    let output = json!({
        "library": library,
        "function": function,
        "args": args_yaml,
        "return": return_yaml,
        "outputs": outputs_yaml,
    });

    println!("{}", serde_yaml::to_string(&output).unwrap());
}

fn value_to_json(value: &Value) -> serde_json::Value {
    use serde_json::json;

    match value {
        Value::I8(v) => json!(v),
        Value::U8(v) => json!(v),
        Value::I16(v) => json!(v),
        Value::U16(v) => json!(v),
        Value::I32(v) => json!(v),
        Value::U32(v) => json!(v),
        Value::I64(v) => json!(v),
        Value::U64(v) => json!(v),
        Value::Isize(v) => json!(v),
        Value::Usize(v) => json!(v),
        Value::F32(v) => json!(v),
        Value::F64(v) => json!(v),
        Value::Ptr(v) => json!(format!("{:p}", v)),
        Value::CStr(s) => json!(s),
        Value::Array { values, .. } => {
            let arr: Vec<_> = values.iter().map(value_to_json).collect();
            json!(arr)
        }
        Value::Callback { signature, .. } => json!(signature),
    }
}
