use std::cell::{Cell, RefCell};
use std::str::CharIndices;

const LINE_SKIP_THRESOLD: usize = 10;
const HIGHER_LINE_SKIP_THRESOLD: usize = 100;
const EXTRA_HIGHER_LINE_SKIP_THRESOLD: usize = 1_000;

/// Contains source text and line locations.
pub struct SourceText {
    pub contents: String,
    processed_lines: Cell<bool>,

    /// Collection of ascending line number *skips* used
    /// for optimizing retrieval of line numbers or line offsets.
    pub(crate) line_skips: RefCell<Vec<LineSkip>>,
    pub(crate) line_skips_counter: Cell<usize>,

    /// Collection used before `line_skips` in line lookups
    /// to skip lines in a higher threshold.
    pub(crate) higher_line_skips: RefCell<Vec<HigherLineSkip>>,
    pub(crate) higher_line_skips_counter: Cell<usize>,

    /// Collection used before `higher_line_skips` in line lookups
    /// to skip lines in an extra higher threshold.
    pub(crate) extra_higher_line_skips: RefCell<Vec<HigherLineSkip>>,
    pub(crate) extra_higher_line_skips_counter: Cell<usize>
}

impl SourceText {
    pub fn new(contents: String) -> Self {
        Self {
            contents,
            processed_lines: Cell::new(false),
            line_skips: RefCell::new(vec![LineSkip { offset: 0, line_number: 1 }]),
            line_skips_counter: Cell::new(0),
            higher_line_skips: RefCell::new(vec![HigherLineSkip { skip_index: 0, offset: 0, line_number: 1 }]),
            higher_line_skips_counter: Cell::new(0),
            extra_higher_line_skips: RefCell::new(vec![HigherLineSkip { skip_index: 0, offset: 0, line_number: 1 }]),
            extra_higher_line_skips_counter: Cell::new(0),
        }
    }

    fn process_lines(&self) {
        if self.processed_lines.get() {
            return;
        }
        self.processed_lines.set(true);
        let mut s = CharacterReader::from(&self.contents);
        let mut line: usize = 1;
        while s.has_remaining() {
            let ch = s.next_or_zero();
            if CharacterValidator::is_line_terminator(ch) {
                if ch == '\r' && s.peek_or_zero() == '\n' {
                    s.next();
                }
                line += 1;
                self.push_line_skip(line, s.index());
            }
        }
    }

    fn push_line_skip(&self, line_number: usize, offset: usize) {
        let counter = self.line_skips_counter.get();
        if counter == LINE_SKIP_THRESOLD {
            self.line_skips.borrow_mut().push(LineSkip { line_number, offset });
            self.line_skips_counter.set(0);
        } else {
            self.line_skips_counter.set(counter + 1);
        }

        let counter = self.higher_line_skips_counter.get();
        if counter == HIGHER_LINE_SKIP_THRESOLD {
            self.higher_line_skips.borrow_mut().push(HigherLineSkip { skip_index: self.line_skips.borrow().len() - 1, line_number, offset });
            self.higher_line_skips_counter.set(0);
        } else {
            self.higher_line_skips_counter.set(counter + 1);
        }

        let counter = self.extra_higher_line_skips_counter.get();
        if counter == EXTRA_HIGHER_LINE_SKIP_THRESOLD {
            self.extra_higher_line_skips.borrow_mut().push(HigherLineSkip { skip_index: self.higher_line_skips.borrow().len() - 1, line_number, offset });
            self.extra_higher_line_skips_counter.set(0);
        } else {
            self.extra_higher_line_skips_counter.set(counter + 1);
        }
    }

    /// Retrieves line number from an offset. The resulting line number
    /// is counted from one.
    pub fn get_line_number(&self, offset: usize) -> usize {
        self.process_lines();

        // Extra higher line skips
        let mut last_skip = HigherLineSkip { skip_index: 0, offset: 0, line_number: 1 };
        let skips = self.extra_higher_line_skips.borrow();
        let mut skips = skips.iter();
        while let Some(skip_1) = skips.next() {
            if offset < skip_1.offset {
                break;
            }
            last_skip = *skip_1;
        }

        // Higher line skips
        let skips = self.higher_line_skips.borrow();
        let mut skips = skips[last_skip.skip_index..].iter();
        let mut last_skip = skips.next().unwrap();
        while let Some(skip_1) = skips.next() {
            if offset < skip_1.offset {
                break;
            }
            last_skip = skip_1;
        }

        // Line skips
        let skips = self.line_skips.borrow();
        let mut skips = skips[last_skip.skip_index..].iter();
        let mut last_skip = skips.next().unwrap();
        while let Some(skip_1) = skips.next() {
            if offset < skip_1.offset {
                break;
            }
            last_skip = skip_1;
        }

        let mut current_line = last_skip.line_number;
        let mut characters = CharacterReader::from(&self.contents[last_skip.offset..]);
        while last_skip.offset + characters.index() < offset {
            let ch_1 = characters.next();
            if let Some(ch_1) = ch_1 {
                if CharacterValidator::is_line_terminator(ch_1) {
                    if ch_1 == '\r' && characters.peek_or_zero() == '\n' {
                        characters.next();
                    }
                    current_line += 1;
                }
            } else {
                break;
            }
        }
        current_line
    }

    /// Retrieves offset from line number (counted from one).
    pub fn get_line_offset(&self, line: usize) -> Option<usize> {
        self.process_lines();

        // Extra higher line skips
        let mut last_skip = HigherLineSkip { skip_index: 0, offset: 0, line_number: 1 };
        let skips = self.extra_higher_line_skips.borrow();
        let mut skips = skips.iter();
        while let Some(skip_1) = skips.next() {
            if line < skip_1.line_number {
                break;
            }
            last_skip = *skip_1;
        }

        // Higher line skips
        let skips = self.higher_line_skips.borrow();
        let mut skips = skips[last_skip.skip_index..].iter();
        let mut last_skip = skips.next().unwrap();
        while let Some(skip_1) = skips.next() {
            if line < skip_1.line_number {
                break;
            }
            last_skip = skip_1;
        }

        // Line skips
        let skips = self.line_skips.borrow();
        let mut skips = skips[last_skip.skip_index..].iter();
        let mut last_skip = skips.next().unwrap();
        while let Some(skip_1) = skips.next() {
            if line < skip_1.line_number {
                break;
            }
            last_skip = skip_1;
        }

        let mut current_line = last_skip.line_number;
        let mut characters = CharacterReader::from(&self.contents[last_skip.offset..]);
        while current_line != line {
            let ch_1 = characters.next();
            if let Some(ch_1) = ch_1 {
                if CharacterValidator::is_line_terminator(ch_1) {
                    if ch_1 == '\r' && characters.peek_or_zero() == '\n' {
                        characters.next();
                    }
                    current_line += 1;
                }
            } else {
                return None;
            }
        }
        Some(last_skip.offset + characters.index())
    }

    /// Retrieves the offset from the corresponding line of an offset.
    pub fn get_line_offset_from_offset(&self, offset: usize) -> usize {
        self.process_lines();

        // Extra higher line skips
        let mut last_skip = HigherLineSkip { skip_index: 0, offset: 0, line_number: 1 };
        let skips = self.extra_higher_line_skips.borrow();
        let mut skips = skips.iter();
        while let Some(skip_1) = skips.next() {
            if offset < skip_1.offset {
                break;
            }
            last_skip = *skip_1;
        }

        // Higher line skips
        let skips = self.higher_line_skips.borrow();
        let mut skips = skips[last_skip.skip_index..].iter();
        let mut last_skip = skips.next().unwrap();
        while let Some(skip_1) = skips.next() {
            if offset < skip_1.offset {
                break;
            }
            last_skip = skip_1;
        }

        // Line skips
        let skips = self.line_skips.borrow();
        let mut skips = skips[last_skip.skip_index..].iter();
        let mut last_skip = skips.next().unwrap();
        while let Some(skip_1) = skips.next() {
            if offset < skip_1.offset {
                break;
            }
            last_skip = skip_1;
        }

        let mut current_line_offset = last_skip.offset;
        let mut characters = CharacterReader::from(&self.contents[last_skip.offset..]);
        while last_skip.offset + characters.index() < offset {
            let ch_1 = characters.next();
            if let Some(ch_1) = ch_1 {
                if CharacterValidator::is_line_terminator(ch_1) {
                    if ch_1 == '\r' && characters.peek_or_zero() == '\n' {
                        characters.next();
                    }
                    current_line_offset = last_skip.offset + characters.index();
                }
            } else {
                break;
            }
        }
        current_line_offset
    }

    /// Returns the zero based column of an offset.
    pub fn get_column(&self, offset: usize) -> usize {
        self.process_lines();

        let line_offset = self.get_line_offset_from_offset(offset);
        let target_offset = offset;
        if line_offset > target_offset {
            return 0;
        }
        let mut i = 0;
        for _ in self.contents[line_offset..target_offset].chars() {
            i += 1;
        }
        i
    }
}

#[derive(Copy, Clone)]
struct LineSkip {
    /// Line offset.
    pub offset: usize,
    /// Line number counting from one.
    pub line_number: usize,
}

#[derive(Copy, Clone)]
struct HigherLineSkip {
    /// Index to a `LineSkip`, or another `HigherLineSkip` in the case
    /// of extra higher line skips.
    pub skip_index: usize,
    /// Line offset.
    pub offset: usize,
    /// Line number counting from one.
    pub line_number: usize,
}

#[derive(Clone)]
struct CharacterReader<'a> {
    length: usize,
    char_indices: CharIndices<'a>,
}

impl<'a> CharacterReader<'a> {
    /// Indicates if there are remaining code points to read.
    pub fn has_remaining(&self) -> bool {
        self.clone().char_indices.next().is_some()
    }

    /// Indicates if the reader has reached the end of the string.
    pub fn _reached_end(&self) -> bool {
        self.clone().char_indices.next().is_none()
    }

    /// Returns the current index in the string.
    pub fn index(&self) -> usize {
        self.clone().char_indices.next().map_or(self.length, |(i, _)| i)
    }

    /// Returns the next code point. If there are no code points
    /// available, returns U+00.
    pub fn next_or_zero(&mut self) -> char {
        self.char_indices.next().map_or('\x00', |(_, cp)| cp)
    }

    /// Peeks the next code point. If there are no code points
    /// available, returns U+00.
    pub fn peek_or_zero(&self) -> char {
        self.clone().next_or_zero()
    }
}

impl<'a> From<&'a str> for CharacterReader<'a> {
    /// Constructs a `CharacterReader` from a string.
    fn from(value: &'a str) -> Self {
        CharacterReader { length: value.len(), char_indices: value.char_indices() }
    }
}

impl<'a> From<&'a String> for CharacterReader<'a> {
    /// Constructs a `CharacterReader` from a string.
    fn from(value: &'a String) -> Self {
        CharacterReader { length: value.len(), char_indices: value.char_indices() }
    }
}

impl<'a> Iterator for CharacterReader<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        self.char_indices.next().map(|(_, cp)| cp)
    }
}

struct CharacterValidator;

impl CharacterValidator {
    pub fn is_line_terminator(ch: char) -> bool {
        ch == '\x0A' || ch == '\x0D' || ch == '\u{2028}' || ch == '\u{2029}'
    }
}

#[cfg(test)]
mod tests {
    use super::SourceText;

    #[test]
    fn test() {
        let text = SourceText::new("foo\r\nbar\r\nqux".into());
        assert_eq!(0, text.get_column(0));
        assert_eq!(0, text.get_column(5));
        assert_eq!(2, text.get_line_number(5));
        assert_eq!(5, text.get_line_offset(2).unwrap());
        assert_eq!(5, text.get_line_offset_from_offset(7));

        let text = SourceText::new("\n".repeat(1_024));
        assert_eq!(1, text.get_line_number(0));
        assert_eq!(2, text.get_line_number(1));
        assert_eq!(1_025, text.get_line_number(1_024));

        let text = SourceText::new("\ndefault xml namespace =\n".into());
        assert_eq!(3, text.get_line_number(25));
        assert_eq!(0, text.get_column(25));
        assert_eq!(2, text.get_line_number(24));
        assert_eq!(23, text.get_column(24));
    }
}