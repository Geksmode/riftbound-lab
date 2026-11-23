//! Générateur aléatoire identique à `random.Random` de CPython 3.13 (MT19937 et les méthodes que le moteur
//! utilise). Même graine = même suite de tirages que le moteur Python : c'est ce qui permet le test
//! différentiel (le Rust doit jouer exactement la même partie que la référence Python).
//!
//! Référence : `Modules/_randommodule.c` (init_genrand, init_by_array, genrand_uint32, random_seed pour un entier)
//! et `Lib/random.py` (_randbelow_with_getrandbits, shuffle, choice, sample, randrange, uniform).

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908_b0df;
const UPPER_MASK: u32 = 0x8000_0000;
const LOWER_MASK: u32 = 0x7fff_ffff;

#[derive(Clone)]
pub struct PyRandom {
    mt: [u32; N],
    mti: usize,
}

impl std::fmt::Debug for PyRandom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PyRandom(mti={})", self.mti)
    }
}

impl PyRandom {
    /// `random.Random(seed)` pour une graine entière : CPython prend la valeur absolue et la découpe en mots de
    /// 32 bits, poids faible d'abord (au moins un mot, 0 pour la graine 0), puis `init_by_array`.
    pub fn new(seed: i128) -> Self {
        let mut n = seed.unsigned_abs();
        let mut key = Vec::new();
        while n > 0 {
            key.push((n & 0xffff_ffff) as u32);
            n >>= 32;
        }
        if key.is_empty() {
            key.push(0);
        }
        let mut r = PyRandom { mt: [0; N], mti: N + 1 };
        r.init_by_array(&key);
        r
    }

    fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            let prev = self.mt[i - 1];
            self.mt[i] = 1_812_433_253u32.wrapping_mul(prev ^ (prev >> 30)).wrapping_add(i as u32);
        }
        self.mti = N;
    }

    fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19_650_218);
        let (mut i, mut j) = (1usize, 0usize);
        let mut k = N.max(key.len());
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
            k -= 1;
        }
        k = N - 1;
        while k > 0 {
            let prev = self.mt[i - 1];
            self.mt[i] = (self.mt[i] ^ (prev ^ (prev >> 30)).wrapping_mul(1_566_083_941)).wrapping_sub(i as u32);
            i += 1;
            if i >= N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x8000_0000;
    }

    /// `genrand_uint32` : un mot de 32 bits.
    pub fn genrand_u32(&mut self) -> u32 {
        if self.mti >= N {
            let mag01 = [0u32, MATRIX_A];
            let mut kk = 0;
            while kk < N - M {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk + M] ^ (y >> 1) ^ mag01[(y & 1) as usize];
                kk += 1;
            }
            while kk < N - 1 {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk + M - N] ^ (y >> 1) ^ mag01[(y & 1) as usize];
                kk += 1;
            }
            let y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
            self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ mag01[(y & 1) as usize];
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^= y >> 18;
        y
    }

    /// `getrandbits(k)` pour k ≤ 64 (le moteur ne demande jamais plus).
    pub fn getrandbits(&mut self, k: u32) -> u64 {
        assert!(k <= 64, "getrandbits: k > 64 non porté");
        if k == 0 {
            return 0;
        }
        if k <= 32 {
            return (self.genrand_u32() >> (32 - k)) as u64;
        }
        // mots de poids faible d'abord ; le dernier mot est tronqué à ses bits de poids fort
        let lo = self.genrand_u32() as u64;
        let hi = (self.genrand_u32() >> (64 - k)) as u64;
        lo | (hi << 32)
    }

    /// `_randbelow_with_getrandbits(n)` : entier uniforme dans [0, n).
    pub fn randbelow(&mut self, n: u64) -> u64 {
        assert!(n > 0, "randbelow(0)");
        let k = 64 - n.leading_zeros();
        let mut r = self.getrandbits(k);
        while r >= n {
            r = self.getrandbits(k);
        }
        r
    }

    /// `randrange(stop)`.
    pub fn randrange(&mut self, stop: i64) -> i64 {
        assert!(stop > 0, "empty range for randrange()");
        self.randbelow(stop as u64) as i64
    }

    /// `randrange(start, stop)` (pas de 1).
    pub fn randrange2(&mut self, start: i64, stop: i64) -> i64 {
        assert!(stop > start, "empty range for randrange()");
        start + self.randbelow((stop - start) as u64) as i64
    }

    /// `randint(a, b)` = `randrange(a, b + 1)`.
    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        self.randrange2(a, b + 1)
    }

    /// `random()` : flottant dans [0, 1) sur 53 bits.
    pub fn random(&mut self) -> f64 {
        let a = (self.genrand_u32() >> 5) as f64;
        let b = (self.genrand_u32() >> 6) as f64;
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// `uniform(a, b)`.
    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// `choice(seq)` : renvoie l'indice choisi (l'appelant indexe sa séquence).
    pub fn choice_index(&mut self, len: usize) -> usize {
        assert!(len > 0, "Cannot choose from an empty sequence");
        self.randbelow(len as u64) as usize
    }

    pub fn choice<'a, T>(&mut self, seq: &'a [T]) -> &'a T {
        &seq[self.choice_index(seq.len())]
    }

    /// `shuffle(x)` (Fisher-Yates de la fin vers le début, comme CPython).
    pub fn shuffle<T>(&mut self, x: &mut [T]) {
        for i in (1..x.len()).rev() {
            let j = self.randbelow(i as u64 + 1) as usize;
            x.swap(i, j);
        }
    }

    /// `sample(population, k)` : renvoie les indices choisis, dans l'ordre de `random.sample`.
    pub fn sample_indices(&mut self, n: usize, k: usize) -> Vec<usize> {
        assert!(k <= n, "Sample larger than population or is negative");
        let mut result = vec![0usize; k];
        let mut setsize = 21usize;
        if k > 5 {
            // _ceil(_log(k * 3, 4)) : math.log(x, b) = log(x) / log(b) en flottants, comme CPython
            let e = ((k as f64 * 3.0).ln() / 4f64.ln()).ceil() as u32;
            setsize += 4usize.pow(e);
        }
        if n <= setsize {
            let mut pool: Vec<usize> = (0..n).collect();
            for (i, slot) in result.iter_mut().enumerate() {
                let j = self.randbelow((n - i) as u64) as usize;
                *slot = pool[j];
                pool[j] = pool[n - i - 1];
            }
        } else {
            let mut selected = std::collections::BTreeSet::new();
            for slot in result.iter_mut() {
                let mut j = self.randbelow(n as u64) as usize;
                while selected.contains(&j) {
                    j = self.randbelow(n as u64) as usize;
                }
                selected.insert(j);
                *slot = j;
            }
        }
        result
    }

    pub fn sample<T: Clone>(&mut self, population: &[T], k: usize) -> Vec<T> {
        self.sample_indices(population.len(), k).into_iter().map(|i| population[i].clone()).collect()
    }
}
