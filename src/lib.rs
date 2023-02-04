use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::ops::{Add, Index, IndexMut};

use rand::seq::SliceRandom;
use rand::Rng;

pub mod node;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

impl Add<Point> for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl Display for Point {
    fn fmt(&self, formatter: &mut Formatter) -> FmtResult {
        write!(formatter, "({}, {})", self.x, self.y)
    }
}

impl Point {
    pub const ZERO: Self = Self { x: 0, y: 0 };
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Symbol {
    Black,
    White,
    Red,
    Green,
    Blue,
    Emerald,
}

impl Default for Symbol {
    fn default() -> Self {
        Symbol::Black
    }
}

impl Symbol {
    pub const PALETTE: &[u8] = &[
        0x00, 0x00, 0x00, // Black
        0xff, 0xf1, 0x38, // White
        0xff, 0x00, 0x4D, // Red
        0x00, 0xe4, 0x36, // Green
        0x29, 0xad, 0xff, // Blue
        0x00, 0x87, 0x51, // Emerald
    ];

    pub fn from_string(string: &str) -> Vec<Option<Self>> {
        string
            .chars()
            .map(|c| match c {
                'B' => Some(Symbol::Black),
                'W' => Some(Symbol::White),
                'R' => Some(Symbol::Red),
                'G' => Some(Symbol::Green),
                'U' => Some(Symbol::Blue),
                'E' => Some(Symbol::Emerald),
                '*' => None,
                c => panic!("unrecognized symbol '{}'", c),
            })
            .collect()
    }

    pub fn palette_index(&self) -> u8 {
        use Symbol::*;
        match self {
            Black => 0,
            White => 1,
            Red => 2,
            Green => 3,
            Blue => 4,
            Emerald => 5,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenericGrid<T> {
    pub width: usize,
    pub height: usize,
    pub grid: Vec<T>,
}

impl<T> Index<Point> for GenericGrid<T> {
    type Output = T;

    fn index(&self, index: Point) -> &T {
        let offset = self.find_offset(index);
        &self.grid[offset]
    }
}

impl<T> IndexMut<Point> for GenericGrid<T> {
    fn index_mut(&mut self, index: Point) -> &mut T {
        let offset = self.find_offset(index);
        &mut self.grid[offset]
    }
}

impl<T> GenericGrid<T> {
    pub fn find_offset(&self, at: Point) -> usize {
        if at.x >= self.width || at.y >= self.height {
            panic!("at {} is out-of-bounds", at);
        }

        at.y * self.width + at.x
    }
}

impl<T: Clone + Default> GenericGrid<T> {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            grid: vec![Default::default(); width * height],
            width,
            height,
        }
    }

    pub fn from_string(string: &[T]) -> Self {
        Self {
            grid: string.to_vec(),
            width: string.len(),
            height: 1,
        }
    }

    pub fn rotate_cw(&self) -> Self {
        let mut grid = Vec::with_capacity(self.width * self.height);

        for x in 0..self.width {
            for y in (0..self.height).rev() {
                let pt = Point { x, y };
                grid.push(self[pt].clone());
            }
        }

        Self {
            grid,
            width: self.height,
            height: self.width,
        }
    }
}

pub type Pattern = GenericGrid<Option<Symbol>>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rule {
    pub find: Pattern,
    pub replace: Pattern,
}

impl Rule {
    pub fn rotate_cw(&self) -> Self {
        Self {
            find: self.find.rotate_cw(),
            replace: self.replace.rotate_cw(),
        }
    }

    pub fn make_rotations(self) -> Vec<Self> {
        let cw = self.rotate_cw();
        let turn = cw.rotate_cw();
        let ccw = turn.rotate_cw();
        vec![self, cw, turn, ccw]
    }

    pub fn from_strings(find: &str, replace: &str) -> Self {
        Self {
            find: Pattern::from_string(&Symbol::from_string(find)),
            replace: Pattern::from_string(&Symbol::from_string(replace)),
        }
    }
}

pub type Grid = GenericGrid<Symbol>;

impl Display for Grid {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        let mut string = String::new();

        for row in self.grid.chunks(self.width) {
            for symbol in row.iter() {
                let character = match symbol {
                    Symbol::Black => 'B',
                    Symbol::White => 'W',
                    Symbol::Red => 'R',
                    Symbol::Green => 'G',
                    Symbol::Blue => 'U',
                    Symbol::Emerald => 'E',
                };

                string.push(character);
            }
            string.push('\n');
        }

        write!(fmt, "{}", string)
    }
}

impl Grid {
    pub fn assert_pattern_fit(&self, pattern: &Pattern, at: Point) {
        if pattern.width + at.x > self.width || pattern.width + at.y > self.height {
            panic!("pattern is out-of-bounds");
        }
    }

    pub fn test_match(&self, pattern: &Pattern, at: Point) -> bool {
        self.assert_pattern_fit(pattern, at);

        for x in 0..pattern.width {
            for y in 0..pattern.height {
                let test_pt = Point { x, y };
                if let Some(expected) = pattern[test_pt] {
                    let grid_pt = test_pt + at;
                    if expected != self[grid_pt] {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub fn apply_pattern(&mut self, pattern: &Pattern, at: Point) {
        self.assert_pattern_fit(pattern, at);
        for x in 0..pattern.width {
            for y in 0..pattern.height {
                let test_pt = Point { x, y };
                if let Some(new_symbol) = pattern[test_pt] {
                    let grid_pt = test_pt + at;
                    self[grid_pt] = new_symbol;
                }
            }
        }
    }

    pub fn find_matches(&self, pattern: &Pattern) -> Vec<Point> {
        self.assert_pattern_fit(pattern, Point::ZERO);

        let mut found = Vec::new();
        let free_width = self.width - pattern.width - 1;
        let free_height = self.height - pattern.height - 1;

        for x in 0..free_width {
            for y in 0..free_height {
                let test_pt = Point { x, y };
                if self.test_match(pattern, test_pt) {
                    found.push(test_pt);
                }
            }
        }

        found
    }

    pub fn render_gif_frame(&self, tile_size: u16) -> gif::Frame<'static> {
        let width = self.width as u16 * tile_size;
        let height = self.height as u16 * tile_size;
        let mut pixels = vec![0; width as usize * height as usize];
        let mut cursor = 0;
        for y in 0..self.height {
            for _ in 0..tile_size {
                for x in 0..self.width {
                    let test_pt = Point { x, y };
                    let index = self[test_pt].palette_index();
                    let dst_range = cursor..(cursor + tile_size as usize);
                    pixels[dst_range].fill(index);
                    cursor += tile_size as usize;
                }
            }
        }

        gif::Frame::from_indexed_pixels(width, height, &pixels, None)
    }

    #[deprecated]
    pub fn run_step(&mut self, rng: &mut impl Rng, step: &Step) -> bool {
        let mut matched = Vec::new();

        for (idx, rule) in step.rules.iter().enumerate() {
            for at in self.find_matches(&rule.find) {
                matched.push((idx, at));
            }
        }

        if let Some((idx, at)) = matched.choose(rng) {
            self.apply_pattern(&step.rules[*idx].replace, *at);
            true
        } else {
            false
        }
    }
}

pub struct Step {
    pub rules: Vec<Rule>,
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::{rngs::SmallRng, SeedableRng};

    pub fn make_rng() -> SmallRng {
        SmallRng::seed_from_u64(2)
    }

    #[test]
    fn rotate_cw() {
        let grid = Grid::new(16, 4);
        let cw = grid.rotate_cw();
        let turn = cw.rotate_cw();
        let ccw = turn.rotate_cw();
        let full = ccw.rotate_cw();
        assert_eq!(full, grid);
    }

    #[test]
    fn maze_backtracker() {
        let mut rng = make_rng();
        let mut grid = Grid::new(16, 16);
        grid.grid[50] = Symbol::Red;

        let rule_one = Rule::from_strings("RBB", "GGR");
        let step_one = Step {
            rules: rule_one.make_rotations(),
        };

        let rule_two = Rule::from_strings("RGG", "WWR");
        let step_two = Step {
            rules: rule_two.make_rotations(),
        };

        let mut matched = true;
        while matched {
            matched = false;

            while grid.run_step(&mut rng, &step_one) {
                matched = true;
            }

            if grid.run_step(&mut rng, &step_two) {
                matched = true;
            }
        }

        print!("{}", grid);
    }
}
