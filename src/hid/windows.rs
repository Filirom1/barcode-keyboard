use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP,
    VK_RETURN, VK_SHIFT, VK_TAB, VIRTUAL_KEY, VkKeyScanA,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suffix {
    Enter,
    Tab,
    None,
}

pub fn inject_hid_reports_windows(text: &str, suffix: Suffix) -> Result<(), String> {
    for ch in text.chars() {
        send_char(ch)?;
    }

    match suffix {
        Suffix::Enter => send_key(VK_RETURN, false)?,
        Suffix::Tab => send_key(VK_TAB, false)?,
        Suffix::None => {}
    }

    Ok(())
}

fn send_char(ch: char) -> Result<(), String> {
    if !ch.is_ascii() {
        return Err(format!("Non-ASCII character not supported: '{}'", ch));
    }

    unsafe {
        let vk_result = VkKeyScanA(ch as u8);
        if vk_result == -1i16 {
            return Err(format!("Character '{}' has no VK mapping", ch));
        }

        let vk_code = (vk_result & 0xFF) as u16;
        let shift_state = (vk_result >> 8) & 0xFF;
        let needs_shift = (shift_state & 0x01) != 0;

        send_key(VIRTUAL_KEY(vk_code), needs_shift)
    }
}

fn send_key(vk: VIRTUAL_KEY, shift: bool) -> Result<(), String> {
    unsafe {
        let mut inputs = Vec::new();

        if shift {
            inputs.push(create_input(VK_SHIFT, false));
        }

        inputs.push(create_input(vk, false));
        inputs.push(create_input(vk, true));

        if shift {
            inputs.push(create_input(VK_SHIFT, true));
        }

        let result = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);

        if result != inputs.len() as u32 {
            return Err(format!(
                "SendInput failed: sent {}/{} events",
                result,
                inputs.len()
            ));
        }

        Ok(())
    }
}

fn create_input(vk: VIRTUAL_KEY, keyup: bool) -> INPUT {
    let mut flags = KEYBD_EVENT_FLAGS(0);
    if keyup {
        flags |= KEYEVENTF_KEYUP;
    }

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ascii_characters() {
        for ch in b'a'..=b'z' {
            let result = send_char(ch as char);
            assert!(result.is_ok(), "Failed to map character: {}", ch as char);
        }

        for ch in b'A'..=b'Z' {
            let result = send_char(ch as char);
            assert!(result.is_ok(), "Failed to map character: {}", ch as char);
        }

        for ch in b'0'..=b'9' {
            let result = send_char(ch as char);
            assert!(result.is_ok(), "Failed to map character: {}", ch as char);
        }
    }

    #[test]
    fn test_special_chars() {
        let test_chars = vec![' ', '-', '_', '.', ',', '!', '@', '#'];
        for ch in test_chars {
            let result = send_char(ch);
            assert!(
                result.is_ok(),
                "Failed to map special character: '{}'",
                ch
            );
        }
    }

    #[test]
    fn test_non_ascii_rejected() {
        let result = send_char('é');
        assert!(result.is_err(), "Non-ASCII character should be rejected");
    }
}
