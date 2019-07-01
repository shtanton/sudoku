extern crate crossbeam_deque;
extern crate crossbeam_utils;

use std::fmt::Display;
use std::fs;
use std::str::FromStr;
use std::iter;

use crossbeam_deque::{Injector, Stealer, Worker};
use crossbeam_utils::thread;

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

fn find_task<T>(
    local: &Worker<T>,
    global: &Injector<T>,
    stealers: &[Stealer<T>],
) -> Option<T> {
    // Pop a task from the local queue, if not empty.
    local.pop().or_else(|| {
        // Otherwise, we need to look for a task elsewhere.
        iter::repeat_with(|| {
            // Try stealing a batch of tasks from the global queue.
            global.steal_batch_and_pop(local)
                // Or try stealing a task from one of the other threads.
                .or_else(|| stealers.iter().map(|s| s.steal()).collect())
        })
        // Loop while no task was stolen and any steal operation needs to be retried.
        .find(|s| !s.is_retry())
        // Extract the stolen task, if there is one.
        .and_then(|s| s.success())
    })
}

pub fn run (sud_string: String, fmt_fname: String) -> Result<(), String>{
    let groups: Vec<Group> = fs::read_to_string(fmt_fname)
        .map_err(|_| "Error reading format file")?
        .trim()
        .split('\n')
        .map(|g| g.parse::<Group>())
        .collect::<Result<Vec<Group>, String>>()?;

    let suds = Injector::<&str>::new();
    sud_string.split("\n").map(|s| {s.trim()}).for_each(|s| {suds.push(s)});


    thread::scope(|s| {
        let mut threads = Vec::with_capacity(NTHREADS);
        let mut stealers = Vec::with_capacity(NTHREADS);
        (0..NTHREADS).for_each(|_| {
            let queue: Worker<&str> = Worker::new_fifo();
            stealers.push(queue.stealer());
            let stealers = stealers.clone();
            threads.push(s.spawn(|_| {
                let queue = queue;
                let stealers = stealers;
                while let Some(sud) = find_task(&queue, &suds, &stealers) {
                    process_single(sud, &groups).unwrap();
                }
            }));
        });
    }).unwrap();

    Ok(())
}
