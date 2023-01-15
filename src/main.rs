
use rusty_buckets::hash3::hash::map::HashTable;


fn main() {

    const CAPACITY: usize = (1 << 21) - 1;
    const SAMPLE_SIZE: usize = (CAPACITY as f64 * 0.93) as usize;

    benchmarking::warm_up();

    let bench_result = benchmarking::measure_function(|measurer| {

        let mut samples: Box<[usize; SAMPLE_SIZE]> = Box::new([0; SAMPLE_SIZE]);
        for i in 0..SAMPLE_SIZE {
            samples[i] = rand::random::<usize>();
            //samples[i] = i+100;
        }

        //samples.sort();

        //let samples: [usize; SAMPLE_SIZE] = [17892297645547504311, 3887224688403108501, 751982014720306921, 6826701797237034623, 1345369401946882797, 13935654535271135208, 4145353771167126259, 15786910114348623016];

        let mut h: HashTable<usize> = HashTable::<usize>::with_capacity(CAPACITY);

        h.print();

        measurer.measure(|| {
            for i in 0..SAMPLE_SIZE {
                h.put(samples[i], samples[i]);
            }
        });

        h.print();

        //measurer.measure(|| {
            for i in 0..SAMPLE_SIZE {
                match h.get(samples[i]){
                    Some(_) => (),
                    None => panic!("Failed to get key {}", samples[i]),
                }
            }
        //});

        h.print();

    }).unwrap();

    println!("Time elapsed {}", bench_result.elapsed().as_nanos() as f64 / SAMPLE_SIZE as f64);
    

}
