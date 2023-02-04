use rand::Rng;

use super::*;

pub trait Step {
    /// Performs a single step.
    ///
    /// Returns true if an operation was performed, i.e. this rule is not done.
    fn step(&mut self, rng: &mut impl Rng, grid: &mut Grid) -> bool;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnyNode {
    Markov(MarkovNode),
    Sequence(SequenceNode),
    One(OneNode),
    All(AllNode),
    Prl(PrlNode),
}

impl AnyNode {
    pub fn make_state(&self) -> AnyState {
        use AnyNode::*;
        match self {
            Markov(node) => AnyState::Markov(MarkovState {
                children: node.children.iter().map(|n| n.make_state()).collect(),
            }),
            Sequence(node) => AnyState::Sequence(SequenceState {
                children: node.children.iter().map(|n| n.make_state()).collect(),
                index: 0,
            }),
            One(node) => AnyState::One(OneState {
                node: node.to_owned(),
                steps_taken: 0,
            }),
            All(node) => AnyState::All(AllState {
                node: node.to_owned(),
                steps_taken: 0,
            }),
            Prl(node) => AnyState::Prl(PrlState {
                node: node.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkovNode {
    pub children: Vec<AnyNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceNode {
    pub children: Vec<AnyNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OneNode {
    pub rules: Vec<Rule>,
    pub steps: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllNode {
    pub rules: Vec<Rule>,
    pub steps: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrlNode {
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AnyState {
    Markov(MarkovState),
    Sequence(SequenceState),
    One(OneState),
    All(AllState),
    Prl(PrlState),
}

impl Step for AnyState {
    fn step(&mut self, rng: &mut impl Rng, grid: &mut Grid) -> bool {
        use AnyState::*;
        match self {
            Markov(s) => s.step(rng, grid),
            Sequence(s) => s.step(rng, grid),
            One(s) => s.step(rng, grid),
            All(s) => s.step(rng, grid),
            Prl(s) => s.step(rng, grid),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MarkovState {
    pub children: Vec<AnyState>,
}

impl Step for MarkovState {
    fn step(&mut self, rng: &mut impl Rng, grid: &mut Grid) -> bool {
        for child in self.children.iter_mut() {
            if child.step(rng, grid) {
                return true;
            }
        }

        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SequenceState {
    pub children: Vec<AnyState>,
    pub index: usize,
}

impl Step for SequenceState {
    fn step(&mut self, rng: &mut impl Rng, grid: &mut Grid) -> bool {
        while let Some(child) = self.children.get_mut(self.index) {
            if child.step(rng, grid) {
                return true;
            } else {
                self.index += 1;
            }
        }

        return false;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OneState {
    pub node: OneNode,
    pub steps_taken: usize,
}

impl Step for OneState {
    fn step(&mut self, rng: &mut impl Rng, grid: &mut Grid) -> bool {
        if let Some(limit) = self.node.steps {
            if self.steps_taken >= limit {
                return false;
            } else {
                self.steps_taken += 1;
            }
        }

        let mut matched = Vec::new();

        for (idx, rule) in self.node.rules.iter().enumerate() {
            for at in grid.find_matches(&rule.find) {
                matched.push((idx, at));
            }
        }

        if let Some((idx, at)) = matched.choose(rng) {
            grid.apply_pattern(&self.node.rules[*idx].replace, *at);
            true
        } else {
            false
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AllState {
    pub node: AllNode,
    pub steps_taken: usize,
}

impl Step for AllState {
    fn step(&mut self, rng: &mut impl Rng, grid: &mut Grid) -> bool {
        if let Some(limit) = self.node.steps {
            if self.steps_taken >= limit {
                return false;
            } else {
                self.steps_taken += 1;
            }
        }

        let mut matched = Vec::new();

        for (idx, rule) in self.node.rules.iter().enumerate() {
            for at in grid.find_matches(&rule.find) {
                matched.push((idx, at));
            }
        }

        if matched.is_empty() {
            return false;
        }

        matched.shuffle(rng);

        for (idx, at) in matched {
            let rule = &self.node.rules[idx];
            if grid.test_match(&rule.find, at) {
                grid.apply_pattern(&rule.replace, at);
            }
        }

        true
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrlState {
    pub node: PrlNode,
}

impl Step for PrlState {
    fn step(&mut self, rng: &mut impl Rng, grid: &mut Grid) -> bool {
        let mut matched = Vec::new();

        for (idx, rule) in self.node.rules.iter().enumerate() {
            for at in grid.find_matches(&rule.find) {
                matched.push((idx, at));
            }
        }

        if matched.is_empty() {
            return false;
        }

        matched.shuffle(rng);

        for (idx, at) in matched {
            grid.apply_pattern(&self.node.rules[idx].replace, at);
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn river() {
        use AnyNode::*;
        let model = Sequence(SequenceNode {
            children: vec![
                One(OneNode {
                    rules: vec![Rule::from_strings("B", "W")],
                    steps: Some(1),
                }),
                One(OneNode {
                    rules: vec![Rule::from_strings("B", "R")],
                    steps: Some(1),
                }),
                One(OneNode {
                    rules: vec![
                        Rule::from_strings("RB", "RR").make_rotations(),
                        Rule::from_strings("WB", "WW").make_rotations(),
                    ]
                    .concat(),
                    steps: None,
                }),
                All(AllNode {
                    rules: Rule::from_strings("RW", "UU").make_rotations(),
                    steps: None,
                }),
                All(AllNode {
                    rules: vec![
                        Rule::from_strings("W", "B").make_rotations(),
                        Rule::from_strings("R", "B").make_rotations(),
                    ]
                    .concat(),
                    steps: None,
                }),
                All(AllNode {
                    rules: Rule::from_strings("UB", "UU").make_rotations(),
                    steps: Some(1),
                }),
                All(AllNode {
                    rules: Rule::from_strings("BU/UB", "U*/**").make_rotations(),
                    steps: None,
                }),
                All(AllNode {
                    rules: Rule::from_strings("UB", "*G").make_rotations(),
                    steps: None,
                }),
                One(OneNode {
                    rules: vec![Rule::from_strings("B", "E")],
                    steps: Some(13),
                }),
                One(OneNode {
                    rules: vec![
                        Rule::from_strings("EB", "*E").make_rotations(),
                        Rule::from_strings("GB", "*G").make_rotations(),
                    ]
                    .concat(),
                    steps: None,
                }),
            ],
        });

        let mut grid = Grid::new(128, 128);
        let mut state = model.make_state();
        let mut rng = crate::tests::make_rng();

        let tile_size = 4;
        let width = grid.width as u16 * tile_size;
        let height = grid.height as u16 * tile_size;
        let mut file = std::fs::File::create("river.gif").unwrap();
        let mut encoder = gif::Encoder::new(&mut file, width, height, Symbol::PALETTE).unwrap();
        encoder.set_repeat(gif::Repeat::Infinite).unwrap();

        let mut frames = Vec::new();
        let mut counter = 0;
        while state.step(&mut rng, &mut grid) {
            println!("Stepping...");

            counter += 1;
            if counter >= 64 {
                counter = 0;
                let mut frame = grid.render_gif_frame(tile_size);
                frame.delay = 2;
                frames.push(frame);
            }
        }

        let mut frame = grid.render_gif_frame(tile_size);
        frame.delay = 1000;
        frames.push(frame);

        for frame in frames {
            encoder.write_frame(&frame).unwrap();
        }

        println!("{}", grid);
    }
}
