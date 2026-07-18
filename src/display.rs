/*
 * in the code 'height' generally refers to the height of a bar in characters
 * and increment refers to which of the 9 block chars best represents the value
 *
 * I have a hunch there is currently a bug with the increments will fix once a more significant prototype exists
 */

use std::io::{self, Write};

const BLOCK: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

pub struct Display {
    width: u16,
    height: u16,
    _string_buffer: String,
    _column_buffers: Vec<Vec<char>>,

    ///the 'height' of each bar
    heights: Vec<u16>,
    ///the 'increment' within the character at the heights height
    increments: Vec<u16>,

    // easier to keep as f32 rather than convert for every input
    _height_multiplier: f32,
    _increment_multiplier: f32,
}

impl Display {
    pub fn new(width: u16, height: u16) -> Display {
        let mut _string_buffer = String::with_capacity((width * height) as usize);
        let mut _column_buffers: Vec<Vec<char>> =
            (0..width).map(|_| vec![' '; height as usize]).collect();

        let heights = vec![0; width as usize];
        let increments = heights.clone();
        let _height_multiplier: f32 = height.into();
        let _increment_multiplier: f32 = (height * BLOCK.len() as u16).into();

        Display {
            width,
            height,
            _string_buffer,
            _column_buffers,
            heights,
            increments,
            _height_multiplier,
            _increment_multiplier,
        }
    }

    pub fn render_frame(&mut self, input: &[f32]) -> &String {
        if (input.len() != self.heights.len()) {
            panic!("graph width cannot currently change at runtime")
        };

        input
            .iter()
            .zip(self.heights.iter_mut().zip(self.increments.iter_mut()))
            .for_each(|(i, (b, ib))| {
                *b = (self._height_multiplier * i) as u16;
                *ib = (self._increment_multiplier * i) as u16 % BLOCK.len() as u16;
            });

        for (buffer, (height, increment)) in self
            ._column_buffers
            .iter_mut()
            .zip(self.heights.iter().zip(self.increments.iter()))
        {
            let mut default = ' ';
            for (char, row) in buffer.iter_mut().zip((0..self.height).rev()) {
                if row == *height {
                    *char = BLOCK[*increment as usize];
                    default = '█';
                } else {
                    *char = default;
                }
            }
        }

        self._string_buffer = String::from("\x1b[H");

        for row in 0..self.height {
            for buffer in self._column_buffers.iter() {
                self._string_buffer.push(buffer[row as usize]);
            }
            self._string_buffer.push_str("\r\n");
        }

        &self._string_buffer.pop();
        &self._string_buffer.pop();
        &self._string_buffer
    }

    pub fn draw_frame(frame: &str) -> io::Result<()> {
        let mut stdout = io::stdout().lock();
        stdout.write_all(frame.as_bytes())?;
        stdout.flush()
    }

    pub fn display(&mut self, input: &[f32]) {
        Display::draw_frame(self.render_frame(input))
            .expect("error drawing the frame, you string was likely malformed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heights() {
        let mut d = Display::new(5, 10);

        let frame = d.render_frame(&vec![0.99_f32, 0.0, 0.7, 0.4, 0.9]);

        let bars = String::from(concat!(
            "\x1b[H",
            "█    \r\n",
            "█   █\r\n",
            "█   █\r\n",
            "█ █ █\r\n",
            "█ █ █\r\n",
            "█ █ █\r\n",
            "█ ███\r\n",
            "█ ███\r\n",
            "█ ███\r\n",
            "█ ███",
        ));

        assert_eq!(&bars, frame)
    }

    #[test]
    fn test_increments_and_heights() {
        let mut d = Display::new(5, 10);

        let frame = d.render_frame(&vec![0.778_f32, 0.0334, 0.7, 0.412, 0.978]);

        let bars = String::from(concat!(
            "\x1b[H",
            "    ▇\r\n",
            "    █\r\n",
            "▇   █\r\n",
            "█ █ █\r\n",
            "█ █ █\r\n",
            "█ █▁█\r\n",
            "█ ███\r\n",
            "█ ███\r\n",
            "█ ███\r\n",
            "█▃███",
        ));

        assert_eq!(&bars, frame)
    }
}
