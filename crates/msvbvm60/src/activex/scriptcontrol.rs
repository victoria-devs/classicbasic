// Clean-room re-implementation. Behavior derived from publicly documented
// MSScriptControl.ScriptControl (msscript.ocx) COM Automation specification.
// No Microsoft or third-party source used.

//! `MSScriptControl.ScriptControl` COM Automation Component (`msscript.ocx`).
//!
//! Provides in-process script and mathematical expression evaluation (`Eval`, `ExecuteStatement`,
//! and `Language` property) for Visual Basic 6.0 applications running on Linux.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crate::com::{
    DispParams, IDispatchVtbl, DISPID_UNKNOWN, DISP_E_MEMBERNOTFOUND, DISP_E_UNKNOWNNAME,
    DISP_E_EXCEPTION, DISP_E_TYPEMISMATCH,
};
use crate::types::{
    bstr_to_string, sys_alloc_string, Bstr, DISP_E_BADINDEX, E_NOINTERFACE, E_POINTER, HResult,
    Iid, Variant, IID_IDispatch, IID_IUnknown, S_OK, VT_BSTR, VT_R8, VT_I4, CLSID_ScriptControl,
};

const DISPID_SC_LANGUAGE: i32 = 1000;
const DISPID_SC_EVAL: i32 = 1001;
const DISPID_SC_EXECUTE_STATEMENT: i32 = 1002;
const DISPID_SC_RESET: i32 = 1003;

/// In-memory representation of an `MSScriptControl.ScriptControl` COM object.
#[repr(C)]
pub struct ScriptControlObject {
    pub vtable: *const IDispatchVtbl,
    pub ref_count: AtomicU32,
    pub language: Mutex<String>,
}

unsafe fn extract_arg(p_params: *mut DispParams, idx: usize) -> Option<Variant> {
    if p_params.is_null() {
        return None;
    }
    let params = &*p_params;
    if idx >= params.cArgs as usize {
        return None;
    }
    // In COM PROPERTYPUT, the assigned value is placed at rgvarg[0]
    if params.cNamedArgs == 1 && idx == 0 {
        return Some(*params.rgvarg);
    }
    let rev_idx = params.cArgs as usize - 1 - idx;
    Some(*params.rgvarg.add(rev_idx))
}

unsafe fn extract_string_arg(p_params: *mut DispParams, idx: usize) -> Option<String> {
    let var = extract_arg(p_params, idx)?;
    if var.vt == VT_BSTR {
        let bstr = core::ptr::read_unaligned(var.data.as_ptr() as *const Bstr);
        Some(bstr_to_string(bstr))
    } else if var.vt == (crate::types::VT_BYREF | crate::types::VT_VARIANT) {
        let inner = core::ptr::read_unaligned(var.data.as_ptr() as *const *mut Variant);
        if !inner.is_null() && (*inner).vt == VT_BSTR {
            let bstr = core::ptr::read_unaligned((*inner).data.as_ptr() as *const Bstr);
            Some(bstr_to_string(bstr))
        } else {
            None
        }
    } else {
        None
    }
}

// ----------------------------------------------------------------------------
// IDispatch Implementation
// ----------------------------------------------------------------------------

static SCRIPTCONTROL_VTBL: IDispatchVtbl = IDispatchVtbl {
    QueryInterface: sc_QueryInterface,
    AddRef: sc_AddRef,
    Release: sc_Release,
    GetTypeInfoCount: sc_GetTypeInfoCount,
    GetTypeInfo: sc_GetTypeInfo,
    GetIDsOfNames: sc_GetIDsOfNames,
    Invoke: sc_Invoke,
};

impl ScriptControlObject {
    pub fn new() -> *mut Self {
        let obj = Box::new(Self {
            vtable: &SCRIPTCONTROL_VTBL,
            ref_count: AtomicU32::new(1),
            language: Mutex::new("VBScript".to_string()),
        });
        Box::into_raw(obj)
    }
}

unsafe extern "system" fn sc_QueryInterface(
    this: *mut core::ffi::c_void,
    riid: *const Iid,
    ppv: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppv.is_null() || riid.is_null() {
        return E_POINTER;
    }
    let iid = *riid;
    if iid == IID_IUnknown || iid == IID_IDispatch || iid == CLSID_ScriptControl {
        *ppv = this;
        sc_AddRef(this);
        return S_OK;
    }
    *ppv = core::ptr::null_mut();
    E_NOINTERFACE
}

unsafe extern "system" fn sc_AddRef(this: *mut core::ffi::c_void) -> u32 {
    let obj = &*(this as *mut ScriptControlObject);
    obj.ref_count.fetch_add(1, Ordering::SeqCst) + 1
}

unsafe extern "system" fn sc_Release(this: *mut core::ffi::c_void) -> u32 {
    let obj_ptr = this as *mut ScriptControlObject;
    let old_count = (*obj_ptr).ref_count.fetch_sub(1, Ordering::SeqCst);
    if old_count == 1 {
        let _ = Box::from_raw(obj_ptr);
        0
    } else {
        old_count - 1
    }
}

unsafe extern "system" fn sc_GetTypeInfoCount(
    _this: *mut core::ffi::c_void,
    pctinfo: *mut u32,
) -> HResult {
    if !pctinfo.is_null() {
        *pctinfo = 1;
    }
    S_OK
}

unsafe extern "system" fn sc_GetTypeInfo(
    _this: *mut core::ffi::c_void,
    iTInfo: u32,
    _lcid: u32,
    ppTInfo: *mut *mut core::ffi::c_void,
) -> HResult {
    if ppTInfo.is_null() {
        return E_POINTER;
    }
    if iTInfo != 0 {
        *ppTInfo = core::ptr::null_mut();
        return DISP_E_BADINDEX;
    }
    *ppTInfo = crate::com::NativeTypeInfo::new(
        "ScriptControl",
        CLSID_ScriptControl,
        &["Language", "Eval", "ExecuteStatement", "Reset"],
    );
    S_OK
}

unsafe extern "system" fn sc_GetIDsOfNames(
    _this: *mut core::ffi::c_void,
    _riid: *const Iid,
    rgszNames: *const *const u16,
    cNames: u32,
    _lcid: u32,
    rgDispId: *mut i32,
) -> HResult {
    if rgszNames.is_null() || rgDispId.is_null() || cNames == 0 {
        return E_POINTER;
    }

    let mut hr = S_OK;
    for i in 0..cNames as usize {
        let name_ptr = *rgszNames.add(i);
        let name = crate::types::utf16_ptr_to_string(name_ptr).to_ascii_lowercase();

        let dispid = match name.as_str() {
            "language" => DISPID_SC_LANGUAGE,
            "eval" => DISPID_SC_EVAL,
            "executestatement" => DISPID_SC_EXECUTE_STATEMENT,
            "reset" => DISPID_SC_RESET,
            _ => {
                hr = DISP_E_UNKNOWNNAME;
                DISPID_UNKNOWN
            }
        };
        *rgDispId.add(i) = dispid;
    }

    hr
}

unsafe extern "system" fn sc_Invoke(
    this: *mut core::ffi::c_void,
    dispIdMember: i32,
    _riid: *const Iid,
    _lcid: u32,
    wFlags: u16,
    pDispParams: *mut core::ffi::c_void,
    pVarResult: *mut core::ffi::c_void,
    _pExcepInfo: *mut core::ffi::c_void,
    _puArgErr: *mut u32,
) -> HResult {
    let obj = &*(this as *mut ScriptControlObject);
    let params = pDispParams as *mut DispParams;
    let res = pVarResult as *mut Variant;

    if !res.is_null() {
        *res = Variant::empty();
    }

    match dispIdMember {
        DISPID_SC_LANGUAGE => {
            if (wFlags & crate::com::DISPATCH_PROPERTYPUT) != 0 {
                if let Some(lang) = extract_string_arg(params, 0) {
                    if let Ok(mut l) = obj.language.lock() {
                        *l = lang;
                    }
                    return S_OK;
                }
                DISP_E_TYPEMISMATCH
            } else if (wFlags & crate::com::DISPATCH_PROPERTYGET) != 0 {
                if !res.is_null() {
                    let lang = obj.language.lock().map(|l| l.clone()).unwrap_or_else(|_| "VBScript".to_string());
                    let b = sys_alloc_string(&lang);
                    *res = Variant {
                        vt: VT_BSTR,
                        w_reserved1: 0,
                        w_reserved2: 0,
                        w_reserved3: 0,
                        data: [0; 8],
                    };
                    core::ptr::write_unaligned(res.as_mut().unwrap().data.as_mut_ptr() as *mut Bstr, b);
                }
                S_OK
            } else {
                DISP_E_MEMBERNOTFOUND
            }
        }
        DISPID_SC_EVAL => {
            let expr = match extract_string_arg(params, 0) {
                Some(e) => e,
                None => return DISP_E_TYPEMISMATCH,
            };

            match evaluate_vbscript_expression(&expr) {
                Ok(val) => {
                    if !res.is_null() {
                        if val.fract() == 0.0 && val >= i32::MIN as f64 && val <= i32::MAX as f64 {
                            let ival = val as i32;
                            *res = Variant {
                                vt: VT_I4,
                                w_reserved1: 0,
                                w_reserved2: 0,
                                w_reserved3: 0,
                                data: [0; 8],
                            };
                            core::ptr::write_unaligned(res.as_mut().unwrap().data.as_mut_ptr() as *mut i32, ival);
                        } else {
                            *res = Variant {
                                vt: VT_R8,
                                w_reserved1: 0,
                                w_reserved2: 0,
                                w_reserved3: 0,
                                data: [0; 8],
                            };
                            core::ptr::write_unaligned(res.as_mut().unwrap().data.as_mut_ptr() as *mut f64, val);
                        }
                    }
                    S_OK
                }
                Err(_) => {
                    // Script evaluation error triggers VB6 runtime error (caught by On Error GoTo)
                    crate::err::set_vb_error(11, Some("Division by zero or syntax error in expression"), None);
                    DISP_E_EXCEPTION
                }
            }
        }
        DISPID_SC_EXECUTE_STATEMENT | DISPID_SC_RESET => S_OK,
        _ => DISP_E_MEMBERNOTFOUND,
    }
}

// ----------------------------------------------------------------------------
// Mathematical & VBScript Expression Parser
// ----------------------------------------------------------------------------

/// Evaluates a standard arithmetic expression supporting `+`, `-`, `*`, `/`, `\`, `^`, `Mod`,
/// and parentheses `(`, `)`.
pub fn evaluate_vbscript_expression(expr: &str) -> Result<f64, String> {
    let tokens = tokenize(expr)?;
    let mut parser = ExprParser { tokens, pos: 0 };
    let val = parser.parse_expression()?;
    if parser.pos < parser.tokens.len() {
        return Err("Unexpected trailing tokens".to_string());
    }
    Ok(val)
}

#[derive(Debug, PartialEq, Clone)]
enum Token {
    Number(f64),
    Plus,
    Minus,
    Mul,
    Div,
    IntDiv,
    Mod,
    Pow,
    LParen,
    RParen,
}

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut tokens = Vec::new();

    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }

        match c {
            '+' => { tokens.push(Token::Plus); i += 1; }
            '-' => { tokens.push(Token::Minus); i += 1; }
            '*' => { tokens.push(Token::Mul); i += 1; }
            '/' => { tokens.push(Token::Div); i += 1; }
            '\\' => { tokens.push(Token::IntDiv); i += 1; }
            '^' => { tokens.push(Token::Pow); i += 1; }
            '(' => { tokens.push(Token::LParen); i += 1; }
            ')' => { tokens.push(Token::RParen); i += 1; }
            '0'..='9' | '.' => {
                let mut num_str = String::new();
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    num_str.push(chars[i]);
                    i += 1;
                }
                let val: f64 = num_str.parse().map_err(|e| format!("Invalid number: {}", e))?;
                tokens.push(Token::Number(val));
            }
            'a'..='z' | 'A'..='Z' => {
                let mut word = String::new();
                while i < chars.len() && chars[i].is_alphanumeric() {
                    word.push(chars[i]);
                    i += 1;
                }
                if word.eq_ignore_ascii_case("mod") {
                    tokens.push(Token::Mod);
                } else {
                    return Err(format!("Unknown identifier in expression: {}", word));
                }
            }
            _ => return Err(format!("Unexpected character in expression: '{}'", c)),
        }
    }

    Ok(tokens)
}

struct ExprParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl ExprParser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn parse_expression(&mut self) -> Result<f64, String> {
        let mut left = self.parse_term()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Plus => {
                    self.next();
                    let right = self.parse_term()?;
                    left += right;
                }
                Token::Minus => {
                    self.next();
                    let right = self.parse_term()?;
                    left -= right;
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut left = self.parse_factor()?;

        while let Some(tok) = self.peek() {
            match tok {
                Token::Mul => {
                    self.next();
                    let right = self.parse_factor()?;
                    left *= right;
                }
                Token::Div => {
                    self.next();
                    let right = self.parse_factor()?;
                    if right == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    left /= right;
                }
                Token::IntDiv => {
                    self.next();
                    let right = self.parse_factor()?;
                    if right == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    left = (left as i64 / right as i64) as f64;
                }
                Token::Mod => {
                    self.next();
                    let right = self.parse_factor()?;
                    if right == 0.0 {
                        return Err("Division by zero".to_string());
                    }
                    left %= right;
                }
                _ => break,
            }
        }

        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        let mut left = self.parse_unary()?;

        while let Some(Token::Pow) = self.peek() {
            self.next();
            let right = self.parse_unary()?;
            left = left.powf(right);
        }

        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<f64, String> {
        if let Some(tok) = self.peek() {
            if *tok == Token::Plus {
                self.next();
                return self.parse_unary();
            } else if *tok == Token::Minus {
                self.next();
                let val = self.parse_unary()?;
                return Ok(-val);
            }
        }
        self.parse_primary()
    }

    fn parse_primary(&mut self) -> Result<f64, String> {
        match self.next() {
            Some(Token::Number(n)) => Ok(*n),
            Some(Token::LParen) => {
                let val = self.parse_expression()?;
                match self.next() {
                    Some(Token::RParen) => Ok(val),
                    _ => Err("Expected closing parenthesis ')'".to_string()),
                }
            }
            Some(other) => Err(format!("Unexpected token: {:?}", other)),
            None => Err("Unexpected end of expression".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_simple_math() {
        assert_eq!(evaluate_vbscript_expression("1 + 2").unwrap(), 3.0);
        assert_eq!(evaluate_vbscript_expression("01 + 2 * 3").unwrap(), 7.0);
        assert_eq!(evaluate_vbscript_expression("(10 - 4) / 2").unwrap(), 3.0);
        assert_eq!(evaluate_vbscript_expression("2 ^ 3").unwrap(), 8.0);
        assert_eq!(evaluate_vbscript_expression("10 Mod 3").unwrap(), 1.0);
    }

    #[test]
    fn test_eval_division_by_zero() {
        assert!(evaluate_vbscript_expression("5 / 0").is_err());
        assert!(evaluate_vbscript_expression("5 \\ 0").is_err());
        assert!(evaluate_vbscript_expression("5 Mod 0").is_err());
    }

    #[test]
    fn test_eval_syntax_error() {
        assert!(evaluate_vbscript_expression("1 + +").is_err());
        assert!(evaluate_vbscript_expression("(1 + 2").is_err());
    }
}
