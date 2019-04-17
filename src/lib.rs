use std::fmt::Display;
use std::fs;
use std::str::FromStr;

#[derive(Clone)]
struct Group {
    cells: Vec<u8>,
}

impl FromStr for Group {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let indices: Vec<u8> = s
            .split(',')
            .map(|c| c.parse::<u8>())
            .collect::<Result<Vec<u8>, core::num::ParseIntError>>()
            .map_err(|_| "Invalid format file")?;
        Ok(Group { cells: indices })
    }
}

#[derive(Clone)]
struct Cell<'a> {
    value: u8,
    groups: Vec<&'a Group>,
}

impl<'a> Cell<'a> {
    fn new<'b>(value: u8) -> Cell<'b> {
        Cell {
            value,
            groups: Vec::with_capacity(3),
        }
    }
    fn add_group(&mut self, group: &'a Group) {
        self.groups.push(group);
    }
    fn possible_values(&self, cells: &Vec<Cell>) -> Vec<u8> {
        let mut avoid: [bool; 10] = [false; 10];
        for group in self.groups.iter() {
            for index in group.cells.iter() {
                avoid[cells[*index as usize].value as usize] = true;
            }
        }
        let mut size: usize = 0;
        for v in avoid.iter().skip(1) {
            if !*v {
                size += 1;
            }
        }
        let mut possible_values: Vec<u8> = Vec::with_capacity(size);
        for (i, v) in avoid.iter().enumerate().skip(1) {
            if !*v {
                possible_values.push(i as u8);
            }
        }
        possible_values
    }
}

impl<'a> FromStr for Cell<'a> {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Cell::new(
            s.parse::<u8>().map_err(|_| "Invalid sudoku file")?,
        ))
    }
}

impl<'a> Display for Cell<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Clone)]
struct Partial<'a> {
    cells: Vec<Cell<'a>>,
}

impl<'a> Partial<'a> {
    fn new<'b>(mut cells: Vec<Cell<'b>>, groups: &'b Vec<Group>) -> Partial<'b> {
        for group in groups.iter() {
            for index in group.cells.iter() {
                cells[*index as usize].add_group(group);
            }
        }
        Partial { cells }
    }
    fn next_empty(&self) -> Option<usize> {
        let cells_iter = self.cells.iter();
        let index = cells_iter
            .enumerate()
            .find_map(|(i, c)| if c.value == 0 { Some(i) } else { None });
        index
    }
}

impl<'a> Display for Partial<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.cells
                .iter()
                .fold(String::new(), |acc, c| acc + &format!("{}", c) + ",")
        )
    }
}

pub fn run(sudoku: &str, group_fname: &str) -> Result<(), String> {
    let cells: Vec<Cell> = sudoku
        .split(',')
        .map(|c| c.parse::<Cell>())
        .collect::<Result<Vec<Cell>, String>>()?;
    let groups: Vec<Group> = fs::read_to_string(group_fname)
        .map_err(|_| "Error reading format file")?
        .trim()
        .split('\n')
        .map(|g| g.parse::<Group>())
        .collect::<Result<Vec<Group>, String>>()?;
    let mut stack: Vec<Partial> = vec![Partial::new(cells, &groups)];

    while !stack.is_empty() {
        let mut start: Partial = stack.pop().expect("SOMETHING WENT HORRIBLY WRONG");
        match start.next_empty() {
            None => {
                //PRINT
                println!("Solution:");
                println!("{}", start);
            }
            Some(i) => {
                for v in start.cells[i].possible_values(&start.cells).iter() {
                    start.cells[i].value = *v;
                    let next: Partial = start.clone();
                    stack.push(next);
                }
            }
        }
    }

    Ok(())
}
