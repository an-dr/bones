use bones_messages::gfx::TextAlign;

pub(crate) fn aligned_text_x(anchor_x: i32, width: u32, align: TextAlign) -> i32 {
    let width = i32::try_from(width).unwrap_or(i32::MAX);
    match align {
        TextAlign::Left => anchor_x,
        TextAlign::Center => anchor_x.saturating_sub(width / 2),
        TextAlign::Right => anchor_x.saturating_sub(width),
    }
}

#[cfg(test)]
mod tests;
