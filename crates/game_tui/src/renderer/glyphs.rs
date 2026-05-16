pub const DENSITY_RAMP_ASCII: [char; 5] = [' ', '.', ':', '*', '#'];
pub const DENSITY_RAMP_UNICODE: [char; 6] = [' ', '·', '░', '▒', '▓', '█'];
pub const STARFIELD_RAMP: [char; 5] = ['·', '•', '✦', '✶', '✷'];

pub fn ramp_pick(ramp: &[char], intensity: u8) -> char {
    if ramp.is_empty() {
        return ' ';
    }
    let index = usize::from(intensity)
        .saturating_mul(ramp.len().saturating_sub(1))
        .checked_div(255)
        .unwrap_or(0);
    ramp[index]
}
