use std::fmt::Display;
use std::fs;
use std::str::FromStr;
use std::thread;
use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;

const NTHREADS: usize = 20;

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
struct Cell {
    value: u8,
    groups: Vec<u8>,
    avoid: [bool; 10],
}

impl Cell {
    fn new(value: u8) -> Cell {
        Cell {
            value,
            groups: Vec::with_capacity(3),
            avoid: [false; 10],
        }
    }
    fn possible_size(&self) -> u8 {
        let mut size: u8 = 0;
        for v in self.avoid.iter().skip(1) {
            if !*v {
                size += 1;
            }
        }
        size
    }
    fn possible_values(&self) -> Vec<u8> {
        let size = self.possible_size() as usize;
        let mut possible_values: Vec<u8> = Vec::with_capacity(size);
        for (i, v) in self.avoid.iter().enumerate().skip(1) {
            if !*v {
                possible_values.push(i as u8);
            }
        }
        possible_values
    }
}

impl<'a> FromStr for Cell {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Cell::new(
            s.parse::<u8>().map_err(|_| "Invalid sudoku file")?,
        ))
    }
}

impl<'a> Display for Cell {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Clone)]
struct Partial<'a> {
    cells: Vec<Cell>,
    groups: &'a Vec<Group>,
}

impl<'a> Partial<'a> {
    fn new<'b>(cells: Vec<Cell>, groups: &'b Vec<Group>) -> Partial<'b> {
        let mut partial = Partial {cells, groups};
        for (index, group) in groups.iter().enumerate() {
            for i in group.cells.iter() {
                partial.cells[*i as usize].groups.push(index as u8);
                for iv in group.cells.iter() {
                    let value = partial.cells[*iv as usize].value;
                    partial.cells[*i as usize].avoid[value as usize] = true;
                }
            }
        }
        partial
    }
    fn next_empty(&self) -> Option<usize> {
        let cells_iter = self.cells.iter();
        let index = cells_iter
            .enumerate()
            .filter(|(_, c)| c.value == 0)
            .min_by(|(_, x), (_, y)| x.possible_size().cmp(&y.possible_size()))
            .map(|(i, _)| i);
        index
    }
    fn children(self) -> Vec<Partial<'a>> {
        match self.next_empty() {
            None => {
                //PRINT
                println!("{}", self);
                Vec::new()
            }
            Some(i) => {
                self.cells[i].possible_values().iter().map(|v| {
                    let mut next: Partial = self.clone();
                    let (pre_cells, cell, post_cells) = {
                        let (pre_cells, post_cells) = next.cells.split_at_mut(i);
                        let (cell, post_cells) = post_cells.split_at_mut(1);
                        (pre_cells, &mut cell[0], post_cells)
                    };
                    cell.value = *v;
                    for group_index in cell.groups.iter() {
                        for index in self.groups[*group_index as usize].cells.iter() {
                            if *index < i as u8 {
                                pre_cells[*index as usize].avoid[*v as usize] = true;
                            } else if *index > i as u8 {
                                post_cells[*index as usize - i - 1].avoid[*v as usize] = true;
                            }
                        }
                    }
                    next
                }).collect::<Vec<_>>()
            }
        }
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

fn process_single(sudoku: &str, groups: &Vec<Group>) -> Result<(), String> {
    let cells: Vec<Cell> = sudoku
        .split(',')
        .map(|c| c.parse::<Cell>())
        .collect::<Result<Vec<Cell>, String>>()?;
    let mut stack: Vec<Partial> = vec![Partial::new(cells, groups)];

    while !stack.is_empty() {
        let start: Partial = stack.pop().expect("SOMETHING WENT HORRIBLY WRONG");
        let mut new: Vec<Partial> = start.children();
        stack.append(&mut new);
    }

    Ok(())
}

pub fn run (sud_string: String, fmt_fname: String) -> Result<(), String>{
    let groups: Vec<Group> = fs::read_to_string(fmt_fname)
        .map_err(|_| "Error reading format file")?
        .trim()
        .split('\n')
        .map(|g| g.parse::<Group>())
        .collect::<Result<Vec<Group>, String>>()?;
    let groups = Arc::new(groups);

    let mut suds = sud_string.split("\n").map(|s: &str| {s.trim()});

		let mut threads = Vec::with_capacity(NTHREADS);
		let mut sud_senders = Vec::with_capacity(NTHREADS);
		let (ask, ask_recv) = channel();

		for i in 0..NTHREADS {
			let (sender, recver): (Sender<String>, _) = channel();
			let ask = ask.clone();
			sud_senders.push(Some(sender));
			let groups = groups.clone();
			threads.push(thread::spawn(move || {
				ask.send(i).unwrap();
				for sud in recver.iter() {
					process_single(&sud, &groups).unwrap();
					ask.send(i).unwrap();
				}
			}));
		}
		drop(ask);

		sud_senders.push(None);
		for i in ask_recv.iter() {
			if let Some(sud) = suds.next() {
				if let Some(sud_sender) = &sud_senders[i] {
					sud_sender.send(sud.to_string()).unwrap();
				}
			} else {
				if sud_senders[i].is_some() {
					sud_senders.swap_remove(i);
					sud_senders.push(None);
				}
			}
		}

    Ok(())
}
