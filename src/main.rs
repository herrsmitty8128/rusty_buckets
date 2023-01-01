
use rusty_buckets::hash_table::HashTable;
use std::time::Instant;

fn main() {

    const CAPACITY: usize = 500000; //(1 << 21) - 1;
    const SAMPLE_SIZE: usize = 1000000; //(CAPACITY as f64 * 0.9375) as usize;

    let mut samples: Box<[usize; SAMPLE_SIZE]> = Box::new([0; SAMPLE_SIZE]);
    for i in 0..SAMPLE_SIZE {
        samples[i] = rand::random::<usize>();
    }

    //let samples: [usize; SAMPLE_SIZE] = [4181431450213987588, 16424795377300343152, 5144252277360976470, 16415711117346800178, 8886427907641364327, 4277509713668868880, 3721955201717284576, 16152639097873742298, 9302267724954876824, 8847114503153701479, 8783765832504146105, 5641369418962583989, 713517944603803882, 5369041230681176259, 9561768724747383243, 9162490756612051478];

    //println!("{:?}", samples);

    let mut h: HashTable<usize> = HashTable::<usize>::with_capacity(CAPACITY);

    let mut i: usize = 0;
    let now: Instant = Instant::now();
    while i < SAMPLE_SIZE {
        h.insert(samples[i], samples[i]);
        i += 1;
    }
    let elapsed: usize = now.elapsed().as_nanos() as usize;

    println!("Initial capacity {} actual capacity {}", CAPACITY, h.capacity());
    println!("Initial entries {} actual entries {}", SAMPLE_SIZE, h.count());
    println!("Load factor {}", h.load_factor());
    println!("Avg time to insert {}", elapsed as f64 / SAMPLE_SIZE as f64);

    i = 0;

    let now: Instant = Instant::now();
    while i < SAMPLE_SIZE {
        match h.get(samples[i]){
            Some(_) => (),
            None => panic!("Failed to get key {}", samples[i]),
        }
        i += 1;
    }
    let elapsed: usize = now.elapsed().as_nanos() as usize;
    
    println!("Initial capacity {} actual capacity {}", CAPACITY, h.capacity());
    println!("Initial entries {} actual entries {}", SAMPLE_SIZE, h.count());
    println!("Avg time to lookup {}", elapsed as f64 / SAMPLE_SIZE as f64);

    i = 0;

    let now: Instant = Instant::now();
    while i < SAMPLE_SIZE {
        h.delete(samples[i]);
        i += 1;
    }
    let elapsed: usize = now.elapsed().as_nanos() as usize;
    
    println!("Initial capacity {} actual capacity {}", CAPACITY, h.capacity());
    println!("Initial entries {} actual entries {}", SAMPLE_SIZE, h.count());
    println!("Avg time to delete {}", elapsed as f64 / SAMPLE_SIZE as f64);

    

}
