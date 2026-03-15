use std::{collections::HashMap, fmt::Write};
use thiserror::Error;

use crate::scripting::lua4::{Lua4Function, Lua4Instruction, Lua4Value};

#[derive(Debug, Clone, Error)]
pub enum Lua4VMError {
    #[error("Missing value in stack")]
    MissingStackValue,

    #[error("Global {0} not found")]
    GlobalNotFound(String),

    #[error("Expected value to be a Closure")]
    NotClosure,

    #[error("Expected value to be a Table")]
    NotTable,

    #[error("Table key not found: {0}")]
    TableKeyNotFound(String),

    #[error("Upvalue at index {0} not found")]
    UpvalueNotFound(u32),

    #[error("Unimplemented instruction {0:?}")]
    Unimplemented(Lua4Instruction),
}

pub trait Lua4VMRustClosures {
    fn call_rust_closure(
        &mut self,
        name: &str,
        parameters: Vec<Lua4Value>,
    ) -> Result<Vec<Lua4Value>, Lua4VMError>;
}

#[derive(Default)]
pub struct Lua4VM {
    pub globals: HashMap<String, Lua4Value>,
}

impl Lua4VM {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_global(&mut self, name: String, value: Lua4Value) {
        self.globals.insert(name, value);
    }

    pub fn get_global(&mut self, name: &str) -> Option<&Lua4Value> {
        self.globals.get(name)
    }

    pub fn call_lua_function<T: Lua4VMRustClosures>(
        &mut self,
        rust_closures: &mut T,
        function: &Lua4Function,
        parameters: &[Lua4Value],
    ) -> Result<Vec<Lua4Value>, anyhow::Error> {
        let mut stack = Vec::with_capacity(function.max_stack_size as usize);
        let local_stack_index = stack.len();
        for i in 0..function.num_parameters as usize {
            stack.push(parameters.get(i).cloned().unwrap_or(Lua4Value::Nil));
        }

        let mut pc = 0;
        loop {
            let instruction = function.instructions[pc];
            pc += 1;
            log::trace!(target: "lua", "[{:03}] {:?}", pc, instruction);
            match instruction {
                Lua4Instruction::OP_END => break,
                Lua4Instruction::OP_RETURN(return_stack_index) => {
                    // Leave only results on stack
                    stack.drain(0..local_stack_index + return_stack_index as usize);
                    break;
                }
                Lua4Instruction::OP_CALL(parameter_stack_index, num_results) => {
                    let parameters =
                        stack.split_off(local_stack_index + parameter_stack_index as usize + 1);
                    let closure = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    let mut results = if let Lua4Value::Closure(function, _upvalues) = closure {
                        let function = function.clone();
                        self.call_lua_function(rust_closures, &function, &parameters)?
                    } else if let Lua4Value::RustClosure(function_name) = closure {
                        let mut debug_message = String::new();

                        if log::log_enabled!(target: "lua", log::Level::Debug) {
                            write!(&mut debug_message, "Call rust closure: {}(", function_name)
                                .ok();

                            for paremeter in parameters.iter().take(1) {
                                write!(&mut debug_message, "{:?}", paremeter).ok();
                            }

                            for paremeter in parameters.iter().skip(1) {
                                write!(&mut debug_message, ", {:?}", paremeter).ok();
                            }
                            debug_message.push(')');
                        }

                        let results =
                            rust_closures.call_rust_closure(&function_name, parameters)?;

                        if log::log_enabled!(target: "lua", log::Level::Debug) {
                            write!(&mut debug_message, " = [").ok();

                            for value in results.iter().take(1) {
                                write!(&mut debug_message, "{:?}", value).ok();
                            }

                            for value in results.iter().skip(1) {
                                write!(&mut debug_message, ", {:?}", value).ok();
                            }
                            debug_message.push(']');
                        }

                        log::debug!(target: "lua", "{}", debug_message);

                        results
                    } else {
                        return Err(Lua4VMError::NotClosure.into());
                    };

                    results.reverse();
                    for _ in 0..num_results {
                        stack.push(results.pop().unwrap_or(Lua4Value::Nil));
                    }
                }
                Lua4Instruction::OP_TAILCALL(parameter_stack_index, num_results) => {
                    let parameters =
                        stack.split_off(local_stack_index + parameter_stack_index as usize + 1);
                    let closure = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    let results = if let Lua4Value::Closure(function, _upvalues) = closure {
                        let function = function.clone();
                        self.call_lua_function(rust_closures, &function, &parameters)?
                    } else if let Lua4Value::RustClosure(function_name) = closure {
                        rust_closures.call_rust_closure(&function_name, parameters)?
                    } else {
                        return Err(Lua4VMError::NotClosure.into());
                    };

                    // For tail call, replace the entire stack with results
                    stack.clear();
                    for result in results {
                        stack.push(result);
                    }
                    // Adjust to return correct number of results
                    while stack.len() < num_results as usize {
                        stack.push(Lua4Value::Nil);
                    }
                    break; // Exit the loop to return
                }
                Lua4Instruction::OP_PUSHNIL(count) => {
                    for _ in 0..count {
                        stack.push(Lua4Value::Nil);
                    }
                }
                Lua4Instruction::OP_POP(count) => {
                    for _ in 0..count {
                        stack.pop();
                    }
                }
                Lua4Instruction::OP_PUSHINT(value) => {
                    stack.push(Lua4Value::Number(value as f64));
                }
                Lua4Instruction::OP_PUSHSTRING(kstr) => {
                    stack.push(Lua4Value::String(
                        function.constant_strings[kstr as usize].clone(),
                    ));
                }
                Lua4Instruction::OP_PUSHNUM(knum) => {
                    stack.push(Lua4Value::Number(function.constant_numbers[knum as usize]));
                }
                Lua4Instruction::OP_PUSHNEGNUM(knum) => {
                    stack.push(Lua4Value::Number(-function.constant_numbers[knum as usize]));
                }
                Lua4Instruction::OP_PUSHUPVALUE(index) => {
                    // Push upvalue from the current closure's upvalue list
                    // Upvalues are stored after the local stack area
                    let upvalue_index = local_stack_index + index as usize;
                    let value = stack
                        .get(upvalue_index)
                        .ok_or(Lua4VMError::UpvalueNotFound(index))?
                        .clone();
                    stack.push(value);
                }
                Lua4Instruction::OP_GETLOCAL(index) => {
                    let value = stack
                        .get(local_stack_index + index as usize)
                        .ok_or(Lua4VMError::MissingStackValue)?
                        .clone();
                    stack.push(value);
                }
                Lua4Instruction::OP_GETGLOBAL(kstr) => {
                    let name = &function.constant_strings[kstr as usize];
                    let value = self
                        .get_global(name)
                        .ok_or_else(|| Lua4VMError::GlobalNotFound(name.into()))?
                        .clone();
                    stack.push(value);
                }
                Lua4Instruction::OP_GETTABLE => {
                    // Pop key and table, push table[key]
                    let key = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let table_value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if let Lua4Value::Table { fields, array } = &table_value {
                        let result = if let Lua4Value::String(key_str) = &key {
                            fields.get(key_str).cloned()
                        } else if let Lua4Value::Number(key_num) = &key {
                            let idx = (*key_num as usize).saturating_sub(1);
                            array.get(idx).cloned()
                        } else {
                            None
                        };
                        stack.push(result.unwrap_or(Lua4Value::Nil));
                    } else {
                        return Err(Lua4VMError::NotTable.into());
                    }
                }
                Lua4Instruction::OP_GETDOTTED(kstr) => {
                    // Pop table, push table[field_name]
                    let field_name = function.constant_strings[kstr as usize].clone();
                    let table_value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if let Lua4Value::Table { fields, .. } = &table_value {
                        let result = fields.get(&field_name).cloned();
                        stack.push(result.unwrap_or(Lua4Value::Nil));
                    } else {
                        return Err(Lua4VMError::NotTable.into());
                    }
                }
                Lua4Instruction::OP_GETINDEXED(kstr) => {
                    // Pop array_index and table, push table[array_index]
                    let array_index = function.constant_strings[kstr as usize].clone();
                    let table_value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if let Lua4Value::Table { fields, .. } = &table_value {
                        let result = fields.get(&array_index).cloned();
                        stack.push(result.unwrap_or(Lua4Value::Nil));
                    } else {
                        return Err(Lua4VMError::NotTable.into());
                    }
                }
                Lua4Instruction::OP_PUSHSELF(kstr) => {
                    // Pop table, push (table, table[field_name]) for method call
                    let field_name = function.constant_strings[kstr as usize].clone();
                    let table_value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    // Push table again (as 'self')
                    stack.push(table_value.clone());

                    if let Lua4Value::Table { fields, .. } = &table_value {
                        let method = fields.get(&field_name).cloned();
                        stack.push(method.unwrap_or(Lua4Value::Nil));
                    } else {
                        return Err(Lua4VMError::NotTable.into());
                    }
                }
                Lua4Instruction::OP_CREATETABLE(array_size) => {
                    // Create a new table with specified initial array size
                    let array = Vec::with_capacity(array_size as usize);
                    let table = Lua4Value::Table {
                        fields: HashMap::new(),
                        array,
                    };
                    stack.push(table);
                }
                Lua4Instruction::OP_SETLOCAL(index) => {
                    stack[local_stack_index + index as usize] =
                        stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                }
                Lua4Instruction::OP_SETGLOBAL(kstr) => {
                    self.set_global(
                        function.constant_strings[kstr as usize].clone(),
                        stack.pop().ok_or(Lua4VMError::MissingStackValue)?,
                    );
                }
                Lua4Instruction::OP_SETTABLE(a, b) => {
                    // Pop value, key; set table[a][key] = value where table is at stack[a] and key is in constant_strings[b]
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let key_str = function.constant_strings[b as usize].clone();
                    
                    // Get table at index a (relative to local stack)
                    let table_index = local_stack_index + a as usize;
                    if table_index < stack.len() {
                        if let Lua4Value::Table { fields, .. } = &mut stack[table_index] {
                            fields.insert(key_str, value);
                        } else {
                            return Err(Lua4VMError::NotTable.into());
                        }
                    }
                }
                Lua4Instruction::OP_SETLIST(a, count) => {
                    // Pop count values and set them as array elements in table at stack[a]
                    let table_index = local_stack_index + a as usize;
                    if table_index >= stack.len() {
                        return Err(Lua4VMError::MissingStackValue.into());
                    }
                    
                    // Collect values to set (they're on stack in reverse order)
                    let mut values = Vec::new();
                    for _ in 0..count {
                        values.push(stack.pop().ok_or(Lua4VMError::MissingStackValue)?);
                    }
                    values.reverse();
                    
                    if let Lua4Value::Table { array, .. } = &mut stack[table_index] {
                        for (i, value) in values.into_iter().enumerate() {
                            while array.len() <= i {
                                array.push(Lua4Value::Nil);
                            }
                            array[i] = value;
                        }
                    } else {
                        return Err(Lua4VMError::NotTable.into());
                    }
                }
                Lua4Instruction::OP_SETMAP(n) => {
                    // Pop n pairs of (key, value) and set them in the table on top of stack
                    let table_value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    
                    let mut pairs = Vec::new();
                    for _ in 0..n {
                        let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                        let key = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                        pairs.push((key, value));
                    }
                    
                    match table_value {
                        Lua4Value::Table { mut fields, .. } => {
                            for (key, value) in pairs {
                                if let Lua4Value::String(key_str) = key {
                                    fields.insert(key_str, value);
                                }
                            }
                            stack.push(Lua4Value::Table { fields, array: Vec::new() });
                        }
                        _ => return Err(Lua4VMError::NotTable.into()),
                    }
                }
                Lua4Instruction::OP_ADD => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    
                    let result = match (&lhs, &rhs) {
                        (Lua4Value::Number(a), Lua4Value::Number(b)) => Lua4Value::Number(a + b),
                        (Lua4Value::String(a), Lua4Value::String(b)) => {
                            Lua4Value::String(format!("{}{}", a, b))
                        }
                        _ => Lua4Value::Nil,
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_ADDI(s) => {
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let result = match value {
                        Lua4Value::Number(n) => Lua4Value::Number(n + s as f64),
                        _ => Lua4Value::Nil,
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_SUB => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    
                    let result = match (&lhs, &rhs) {
                        (Lua4Value::Number(a), Lua4Value::Number(b)) => Lua4Value::Number(a - b),
                        _ => Lua4Value::Nil,
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_MULT => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    
                    let result = match (&lhs, &rhs) {
                        (Lua4Value::Number(a), Lua4Value::Number(b)) => Lua4Value::Number(a * b),
                        _ => Lua4Value::Nil,
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_DIV => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    
                    let result = match (&lhs, &rhs) {
                        (Lua4Value::Number(a), Lua4Value::Number(b)) if *b != 0.0 => Lua4Value::Number(a / b),
                        _ => Lua4Value::Nil,
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_POW => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    
                    let result = match (&lhs, &rhs) {
                        (Lua4Value::Number(a), Lua4Value::Number(b)) => Lua4Value::Number(a.powf(*b)),
                        _ => Lua4Value::Nil,
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_CONCAT(count) => {
                    // Pop count strings and concatenate them
                    let mut parts = Vec::new();
                    for _ in 0..count {
                        let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                        let str = match value {
                            Lua4Value::String(s) => s,
                            Lua4Value::Number(n) => n.to_string(),
                            _ => String::new(),
                        };
                        parts.push(str);
                    }
                    parts.reverse();
                    stack.push(Lua4Value::String(parts.join("")));
                }
                Lua4Instruction::OP_MINUS => {
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let result = match value {
                        Lua4Value::Number(n) => Lua4Value::Number(-n),
                        _ => Lua4Value::Nil,
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_NOT => {
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let result = match value {
                        Lua4Value::Nil => Lua4Value::Number(1.0), // true in Lua4 (1.0 = true)
                        _ => Lua4Value::Nil, // false in Lua4 (nil = false)
                    };
                    stack.push(result);
                }
                Lua4Instruction::OP_JMPNE(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs != rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPEQ(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs == rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPLT(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs < rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPLE(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs <= rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPGT(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs > rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPGE(target) => {
                    let rhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;
                    let lhs = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if lhs >= rhs {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPT(target) => {
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if !matches!(value, Lua4Value::Nil) {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPF(target) => {
                    let value = stack.pop().ok_or(Lua4VMError::MissingStackValue)?;

                    if matches!(value, Lua4Value::Nil) {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPONT(target) => {
                    // If value on top of stack is Nil then pop it, else jump
                    let peek_value = stack.last().ok_or(Lua4VMError::MissingStackValue)?;

                    if matches!(peek_value, Lua4Value::Nil) {
                        stack.pop();
                    } else {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMPONF(target) => {
                    // If value on top of stack is not Nil then pop it, else jump
                    let peek_value = stack.last().ok_or(Lua4VMError::MissingStackValue)?;

                    if !matches!(peek_value, Lua4Value::Nil) {
                        stack.pop();
                    } else {
                        pc = (pc as i32 + target) as usize;
                    }
                }
                Lua4Instruction::OP_JMP(target) => {
                    pc = (pc as i32 + target) as usize;
                }
                Lua4Instruction::OP_PUSHNILJMP => {
                    stack.push(Lua4Value::Nil);
                    pc = (pc as i32 + 1) as usize;
                }
                Lua4Instruction::OP_FORPREP(skip) => {
                    // Initialize numeric for loop: for i = init, limit, step do ...
                    // Stack has: init, limit, step (from bottom to top)
                    // Adjust initial value by negative step to prepare for post-increment
                    let step_idx = stack.len() - 1;
                    let limit_idx = stack.len() - 2;
                    let init_idx = stack.len() - 3;
                    
                    if step_idx >= 3 {
                        let step = match &stack[step_idx] {
                            Lua4Value::Number(s) => *s,
                            _ => 1.0,
                        };
                        
                        // Initialize control variable (decrement by step for post-increment semantics)
                        if let Lua4Value::Number(mut init) = stack[init_idx].clone() {
                            init -= step;
                            stack[init_idx] = Lua4Value::Number(init);
                        }
                        
                        // Jump to FORLOOP
                        pc = (pc as i32 + skip) as usize;
                    }
                }
                Lua4Instruction::OP_FORLOOP(backward) => {
                    // Numeric for loop iteration
                    // Stack has: init, limit, step (from bottom to top)
                    let step_idx = stack.len() - 1;
                    let limit_idx = stack.len() - 2;
                    let init_idx = stack.len() - 3;
                    
                    if step_idx >= 3 {
                        let init = match &stack[init_idx] {
                            Lua4Value::Number(i) => *i,
                            _ => 0.0,
                        };
                        let limit = match &stack[limit_idx] {
                            Lua4Value::Number(l) => *l,
                            _ => 0.0,
                        };
                        let step = match &stack[step_idx] {
                            Lua4Value::Number(s) => *s,
                            _ => 1.0,
                        };
                        
                        // Increment control variable
                        let new_init = init + step;
                        stack[init_idx] = Lua4Value::Number(new_init);
                        
                        // Check if we should continue
                        if (step > 0.0 && new_init <= limit) || (step < 0.0 && new_init >= limit) {
                            // Loop body
                            pc = (pc as i32 + backward) as usize;
                        }
                        // else: fall through to continue after loop
                    }
                }
                Lua4Instruction::OP_LFORPREP(skip) => {
                    // Prepare generic for loop (for v in iterator do ...)
                    // Push nil to initialize the iteration
                    stack.push(Lua4Value::Nil);
                    // Jump to LFORLOOP
                    pc = (pc as i32 + skip) as usize;
                }
                Lua4Instruction::OP_LFORLOOP(backward) => {
                    // Generic for loop iteration
                    // Stack has: function, state, control, result
                    let result_idx = stack.len() - 1;
                    let control_idx = stack.len() - 3;
                    
                    if result_idx >= 3 {
                        // Check if result is nil (end of iteration)
                        if matches!(&stack[result_idx], Lua4Value::Nil) {
                            // End of iteration, clean up and exit loop
                            stack.truncate(control_idx);
                        } else {
                            // Continue loop, set control variable to result
                            stack[control_idx] = stack[result_idx].clone();
                            // Jump back to loop body
                            pc = (pc as i32 + backward) as usize;
                        }
                    }
                }
                Lua4Instruction::OP_CLOSURE(kproto, b) => {
                    let upvalues = stack.split_off(stack.len() - b as usize);
                    stack.push(Lua4Value::Closure(
                        function.constant_functions[kproto as usize].clone(),
                        upvalues,
                    ));
                }
                _ => {
                    anyhow::bail!(Lua4VMError::Unimplemented(instruction))
                }
            }
        }

        Ok(stack)
    }

    pub fn call_global_closure<T: Lua4VMRustClosures>(
        &mut self,
        rust_closures: &mut T,
        name: &str,
        parameters: &[Lua4Value],
    ) -> Result<Vec<Lua4Value>, anyhow::Error> {
        let global_value = self
            .get_global(name)
            .ok_or_else(|| Lua4VMError::GlobalNotFound(name.into()))?;

        if let Lua4Value::Closure(function, _upvalues) = global_value {
            let function = function.clone();
            self.call_lua_function(rust_closures, &function, parameters)
        } else {
            Err(Lua4VMError::NotClosure.into())
        }
    }
}
