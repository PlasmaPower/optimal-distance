use std::env;
use std::f32;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufReader, BufRead};

type SIndex = u16;

#[derive(PartialEq)]
struct HeapItem {
    distance: f32,
    path: Vec<SIndex>,
}

impl HeapItem {
    fn new(min_total_dist: f32) -> HeapItem {
        HeapItem {
            distance: min_total_dist,
            path: Vec::new(),
        }
    }

    fn split(&self, new_item: SIndex, dist_diff: f32) -> HeapItem {
        let mut new_path = self.path.clone();
        new_path.push(new_item);
        HeapItem {
            distance: self.distance + dist_diff,
            path: new_path,
        }
    }
}

impl Eq for HeapItem {}

impl Ord for HeapItem {
    fn cmp(&self, other: &HeapItem) -> Ordering {
        // reversed on purpose
        other.distance.partial_cmp(&self.distance).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd<HeapItem> for HeapItem {
    fn partial_cmp(&self, other: &HeapItem) -> Option<Ordering> {
        other.distance.partial_cmp(&self.distance)
    }
}

fn main() {
    let mut args = env::args();
    args.next();
    let file = File::open(args.next().unwrap_or_else(|| "cophenetic_pairs_TT".into())).expect("Failed to open file");
    let mut file = BufReader::new(file);
    let mut buffer = String::new();
    let mut r_items = Vec::new();
    let mut s_items = Vec::new();
    let mut distances = Vec::new();
    let mut file_idx_rs = Vec::new();
    let mut is_header = true;
    let mut min_distance = f32::INFINITY;
    loop {
        buffer.clear();
        file.read_line(&mut buffer).expect("Failed to read file");
        if buffer.is_empty() {
            break;
        }
        buffer.pop(); // '\n'
        let mut items = buffer.split(' ').map(|mut x| {
            if x.ends_with('\r') {
                x = &x[..x.len() - 1]
            }
            if x.starts_with('"') {
                &x[1..x.len() - 1]
            } else {
                x
            }
        });
        if is_header {
            is_header = false;
            let mut len = 0;
            for item in items {
                len += 1;
                if item.starts_with('R') {
                    r_items.push(String::from(item));
                    file_idx_rs.push(Some(false));
                } else if item.starts_with('S') {
                    s_items.push(String::from(item));
                    file_idx_rs.push(Some(true));
                } else {
                    file_idx_rs.push(None);
                }
            }
            distances.resize(len, Vec::new());
        } else {
            let item = match items.next() {
                Some(x) => x,
                None => continue,
            };
            let index = match r_items.iter().position(|x| x == &item) {
                Some(x) => x,
                None => continue,
            };
            let is_s = item.starts_with('S');
            distances[index] = items
                .map(str::parse)
                .enumerate()
                .filter_map(|(i, x)| {
                    let x = x.expect("Failed to parse distance");
                    if let Some(is_other_s) = file_idx_rs[i] {
                        if is_s != is_other_s {
                            if x < min_distance {
                                min_distance = x;
                            }
                            return Some(x);
                        }
                    }
                    None
                })
                .collect::<Vec<f32>>();
        }
    }
    if s_items.len() > (SIndex::max_value() as usize) {
        panic!("There are {} S_ items, but only {} can be indexed", s_items.len(), SIndex::max_value());
    }
    let mut heap = BinaryHeap::new();
    heap.push(HeapItem::new(min_distance * (r_items.len() as f32)));
    while let Some(item) = heap.pop() {
        if item.path.len() >= r_items.len() {
            // Done!
            let max_r_len = r_items.iter().map(String::len).max().expect("No R items");
            let max_s_len = s_items.iter().map(String::len).max().expect("No S items");
            for (r_i, s_i) in item.path.into_iter().enumerate() {
                println!("{:r_w$} -> {:s_w$} = {:.6}", r_items[r_i], s_items[s_i as usize], distances[r_i][s_i as usize], r_w = max_r_len, s_w = max_s_len);
            }
            println!("{}", (0..max_r_len + max_s_len + 15).map(|_| '-').collect::<String>());
            println!("Total distance:{:>1$.6}", item.distance, max_r_len + max_s_len);
            break;
        }
        for i in 0..s_items.len() {
            if !item.path.contains(&(i as u16)) {
                let dist = distances[item.path.len()][i];
                heap.push(item.split(i as u16, dist - min_distance));
            }
        }
    }
}
