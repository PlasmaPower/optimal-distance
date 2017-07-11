use std::env;
use std::f32;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufReader, BufRead};

extern crate fnv;
use fnv::FnvHashSet;

type SIndex = u16;

#[derive(PartialEq)]
struct HeapItem {
    distance: f32,
    path: Vec<SIndex>,
}

impl HeapItem {
    fn new(path: Vec<SIndex>, min_total_dist: f32) -> HeapItem {
        HeapItem {
            distance: min_total_dist,
            path: path,
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

fn bloom_set_hash<I: Iterator<Item = u16>>(it: I) -> u64 {
    // Modified FNV (Fowler–Noll–Vo) hash.
    // Designed to be order independent.
    let mut out = 0;
    for i in it {
        let mut hash = 0xcbf29ce484222325;
        hash = (hash ^ ((i & 0xff) as u64)).wrapping_mul(0x100000001b3);
        hash = (hash ^ ((i >> 8) as u64)).wrapping_mul(0x100000001b3);
        out ^= hash;
    }
    out
}

fn main() {
    let mut args = env::args();
    args.next();
    let file = File::open(args.next().unwrap_or_else(|| "cophenetic_pairs_TT".into())).expect("Failed to open file");
    let mut file = BufReader::new(file);
    let mut buffer = String::new();
    let mut r_items = Vec::new();
    let mut s_items = Vec::new();
    let mut s_indicies = Vec::new();
    let mut distances = Vec::new();
    let mut is_header = true;
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
            for (i, item) in items.into_iter().enumerate() {
                if item.starts_with('R') {
                    r_items.push(String::from(item));
                } else if item.starts_with('S') {
                    s_items.push(String::from(item));
                    s_indicies.push(i);
                }
            }
            distances.resize(r_items.len(), Vec::new());
        } else {
            let item = match items.next() {
                Some(x) => x,
                None => continue,
            };
            if item.starts_with('R') {
                let index = match r_items.iter().position(|x| x == &item) {
                    Some(x) => x,
                    None => continue,
                };
                let items = items.map(str::parse).collect::<Result<Vec<f32>, _>>().expect("Failed to parse float");
                let ref mut distances = distances[index];
                for &idx in s_indicies.iter() {
                    distances.push(items[idx]);
                }
            }
        }
    }
    let mut r_best_s = Vec::new();
    let mut min_step_distance = vec![f32::INFINITY; r_items.len()];
    for i in 0..r_items.len() {
        const N_S: usize = 10;
        let mut distances = distances[i].iter().cloned().enumerate().collect::<Vec<_>>();
        distances.sort_by(|&(_, x), &(_, y)| x.partial_cmp(&y).expect("NaN in file"));
        r_best_s.push(distances.into_iter()
                      .take(N_S)
                      .map(|(n, x)| {
                          if x < min_step_distance[i] {
                              min_step_distance[i] = x;
                          }
                          (n, x)
                      })
                      .map(|(i, _)| i as SIndex)
                      .collect::<Vec<_>>());
    }
    if s_items.len() > (SIndex::max_value() as usize) {
        panic!("There are {} S_ items, but only {} can be indexed", s_items.len(), SIndex::max_value());
    }
    let mut max_explored = 0;
    let mut tried_paths: FnvHashSet<u64> = Default::default();
    let mut heap = BinaryHeap::new();
    heap.push(HeapItem::new(Vec::new(), min_step_distance.iter().sum()));
    while let Some(mut item) = heap.pop() {
        let path_len = item.path.len();
        if path_len >= r_items.len() {
            // Done!
            let max_r_len = r_items.iter().map(String::len).max().expect("No R items");
            let max_s_len = s_items.iter().map(String::len).max().expect("No S items");
            for (r_i, s_i) in item.path.into_iter().enumerate() {
                println!("{:r_w$} -> {:s_w$} = {:.6}", r_items[r_i], s_items[s_i as usize], distances[r_i][s_i as usize], r_w = max_r_len, s_w = max_s_len);
            }
            println!("{}", (0..max_r_len + max_s_len + 15).map(|_| '-').collect::<String>());
            println!("Total distance:{:>1$.6}", item.distance, max_r_len + max_s_len);
            break;
        } else if path_len > max_explored {
            max_explored = path_len;
            println!("Explored depth: {:>3}% ({:>s_w$}/{})", (100*path_len)/r_items.len(), path_len, r_items.len(), s_w = (r_items.len() as f32).log(10.).floor() as usize + 1);
        }
        if !tried_paths.insert(bloom_set_hash(item.path.iter().cloned())) {
            continue;
        }
        if !item.path.is_empty() {
            let idx = path_len - 1;
            let prev_s_i = *item.path.last().unwrap();
            let prev_dist = distances[idx][prev_s_i as usize];
            let mut s_iter = r_best_s[idx].iter();
            let _ = s_iter.find(|&&x| x == prev_s_i);
            let mut new_path = item.path.clone();
            for &s_i in s_iter {
                *new_path.last_mut().unwrap() = s_i;
                if !tried_paths.contains(&bloom_set_hash(new_path.iter().cloned())) {
                    let dist_diff = distances[idx][s_i as usize] - prev_dist;
                    heap.push(HeapItem::new(new_path, item.distance + dist_diff));
                    break;
                }
            }
        }
        item.path.push(0);
        for &s_i in r_best_s[path_len].iter() {
            if !item.path[..path_len].contains(&(s_i as u16)) {
                *item.path.last_mut().unwrap() = s_i;
                if !tried_paths.contains(&bloom_set_hash(item.path.iter().cloned())) {
                    let dist_diff = distances[path_len][s_i as usize] - min_step_distance[path_len];
                    heap.push(HeapItem::new(item.path, item.distance + dist_diff));
                    break;
                }
            }
        }
    }
}
