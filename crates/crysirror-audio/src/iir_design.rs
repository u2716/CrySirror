// Copyright (c) 2026 CrySirror contributors.
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. A copy is available at https://mozilla.org/MPL/2.0/.
//! IIR filter design and biquad realisation in the DSPFilters formulation.
//!
//! Ported from Vinnie Falco's DSPFilters (MIT, `vinniefalco/DSPFilters`
//! acc49170, notice in `dspfilters.LICENSE`): analog prototypes (Butterworth,
//! Chebyshev I/II, Elliptic, Bessel, Legendre), the bilinear low/high/band
//! pass/stop transforms, cascade normalisation, the RBJ cookbook biquads, the
//! custom one/two-pole sections, the parameter tables and the four biquad
//! state forms with the alternating 1e-8 anti-denormal term. The
//! arithmetic keeps the upstream order so a design matches a binary built
//! from that source (KiriKiri `wfTypicalDSP.dll` links it; see
//! crysirror-krkr `wf_plugins`).
use crysirror_core::service::{IirFamily, IirKind, IirStructure};
use std::f64::consts::PI;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct C {
    re: f64,
    im: f64,
}
const fn c(re: f64, im: f64) -> C {
    C { re, im }
}
const INFINITY: C = c(f64::INFINITY, 0.);
impl C {
    fn real(re: f64) -> C {
        c(re, 0.)
    }
    fn polar(r: f64, t: f64) -> C {
        c(r * t.cos(), r * t.sin())
    }
    fn conj(self) -> C {
        c(self.re, -self.im)
    }
    fn norm(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    fn scale(self, k: f64) -> C {
        c(self.re * k, self.im * k)
    }
    /// Principal square root with the signed-zero branch cut of `std::sqrt`.
    fn sqrt(self) -> C {
        if self.re == 0. && self.im == 0. {
            return c(0., self.im);
        }
        let t = ((self.re.abs() + self.abs()) / 2.).sqrt();
        if self.re >= 0. {
            c(t, self.im / (2. * t))
        } else {
            c(self.im.abs() / (2. * t), t.copysign(self.im))
        }
    }
}
impl std::ops::Add for C {
    type Output = C;
    fn add(self, o: C) -> C {
        c(self.re + o.re, self.im + o.im)
    }
}
impl std::ops::Sub for C {
    type Output = C;
    fn sub(self, o: C) -> C {
        c(self.re - o.re, self.im - o.im)
    }
}
impl std::ops::Neg for C {
    type Output = C;
    fn neg(self) -> C {
        c(-self.re, -self.im)
    }
}
impl std::ops::Mul for C {
    type Output = C;
    fn mul(self, o: C) -> C {
        c(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}
impl std::ops::Div for C {
    type Output = C;
    fn div(self, o: C) -> C {
        let d = o.norm();
        c((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
    }
}
/// `addmul(c, v, c1)`: `c + v * c1`.
fn addmul(a: C, v: f64, b: C) -> C {
    c(a.re + v * b.re, a.im + v * b.im)
}

#[derive(Clone, Copy, Debug, Default)]
struct Pair {
    p1: C,
    z1: C,
    p2: C,
    z2: C,
}
impl Pair {
    fn single(&self) -> bool {
        self.p2 == C::default() && self.z2 == C::default()
    }
}
#[derive(Clone, Debug, Default)]
struct Layout {
    pairs: Vec<Pair>,
    poles: usize,
    normal_w: f64,
    normal_gain: f64,
}
impl Layout {
    fn reset(&mut self) {
        self.pairs.clear();
        self.poles = 0;
    }
    fn add(&mut self, pole: C, zero: C) {
        self.pairs.push(Pair { p1: pole, z1: zero, ..Default::default() });
        self.poles += 1;
    }
    fn add_conjugates(&mut self, pole: C, zero: C) {
        self.pairs.push(Pair { p1: pole, z1: zero, p2: pole.conj(), z2: zero.conj() });
        self.poles += 2;
    }
    fn add_pairs(&mut self, poles: (C, C), zeros: (C, C)) {
        self.pairs.push(Pair { p1: poles.0, z1: zeros.0, p2: poles.1, z2: zeros.1 });
        self.poles += 2;
    }
    fn set_normal(&mut self, w: f64, g: f64) {
        self.normal_w = w;
        self.normal_gain = g;
    }
}

// ---- analog prototypes --------------------------------------------------

fn butterworth_lowpass(n: usize) -> Layout {
    let mut l = Layout::default();
    l.set_normal(0., 1.);
    let n2 = 2. * n as f64;
    for i in 0..n / 2 {
        l.add_conjugates(C::polar(1., PI / 2. + (2 * i + 1) as f64 * PI / n2), INFINITY);
    }
    if n & 1 == 1 {
        l.add(C::real(-1.), INFINITY);
    }
    l
}
fn butterworth_lowshelf(n: usize, gain_db: f64) -> Layout {
    let mut l = Layout::default();
    l.set_normal(PI, 1.);
    let n2 = n as f64 * 2.;
    let g = 10f64.powf(gain_db / 20.).powf(1. / n2);
    let (gp, gz) = (-1. / g, -g);
    for i in 1..=n / 2 {
        let theta = PI * (0.5 - (2 * i - 1) as f64 / n2);
        l.add_conjugates(C::polar(gp, theta), C::polar(gz, theta));
    }
    if n & 1 == 1 {
        l.add(C::real(gp), C::real(gz));
    }
    l
}
fn chebyshev1_lowpass(n: usize, ripple_db: f64) -> Layout {
    let mut l = Layout::default();
    let eps = (1. / (-ripple_db * 0.1 * std::f64::consts::LN_10).exp() - 1.).sqrt();
    let v0 = (1. / eps).asinh() / n as f64;
    let (sinh_v0, cosh_v0) = (-v0.sinh(), v0.cosh());
    let n2 = 2. * n as f64;
    for i in 0..n / 2 {
        let k = (2 * i + 1) as f64 - n as f64;
        let a = sinh_v0 * (k * PI / n2).cos();
        let b = cosh_v0 * (k * PI / n2).sin();
        l.add_conjugates(c(a, b), INFINITY);
    }
    if n & 1 == 1 {
        l.add(C::real(sinh_v0), INFINITY);
        l.set_normal(0., 1.);
    } else {
        l.set_normal(0., 10f64.powf(-ripple_db / 20.));
    }
    l
}
/// Orfanidis shelf prototype, shared verbatim by both Chebyshev families.
fn chebyshev_lowshelf(n: usize, gain_db: f64, band_db: f64) -> Layout {
    let mut l = Layout::default();
    l.set_normal(PI, 1.);
    let gain_db = -gain_db;
    let mut band_db = if band_db >= gain_db.abs() { gain_db.abs() } else { band_db };
    if gain_db < 0. {
        band_db = -band_db;
    }
    let g = 10f64.powf(gain_db / 20.);
    let gb = 10f64.powf((gain_db - band_db) / 20.);
    let g0 = 1f64;
    let g0n = g0.powf(1. / n as f64);
    let eps = if gb != g0 { ((g * g - gb * gb) / (gb * gb - g0 * g0)).sqrt() } else { g - 1. };
    let b = (g / eps + gb * (1. + 1. / (eps * eps)).sqrt()).powf(1. / n as f64);
    let u = (b / g0n).ln();
    let v = ((1. / eps + (1. + 1. / (eps * eps)).sqrt()).powf(1. / n as f64)).ln();
    let (sinh_u, sinh_v, cosh_u, cosh_v) = (u.sinh(), v.sinh(), u.cosh(), v.cosh());
    let n2 = 2. * n as f64;
    for i in 1..=n / 2 {
        let a = PI * (2 * i - 1) as f64 / n2;
        let (sn, cs) = (a.sin(), a.cos());
        l.add_conjugates(c(-sn * sinh_u, cs * cosh_u), c(-sn * sinh_v, cs * cosh_v));
    }
    if n & 1 == 1 {
        l.add(C::real(-sinh_u), C::real(-sinh_v));
    }
    l
}
fn chebyshev2_lowpass(n: usize, stop_db: f64) -> Layout {
    let mut l = Layout::default();
    l.set_normal(0., 1.);
    let eps = (1. / ((stop_db * 0.1 * std::f64::consts::LN_10).exp() - 1.)).sqrt();
    let v0 = (1. / eps).asinh() / n as f64;
    let (sinh_v0, cosh_v0) = (-v0.sinh(), v0.cosh());
    let fn_ = PI / (2 * n) as f64;
    let mut k = 1i64;
    for _ in 0..n / 2 {
        let a = sinh_v0 * ((k - n as i64) as f64 * fn_).cos();
        let b = cosh_v0 * ((k - n as i64) as f64 * fn_).sin();
        let d2 = a * a + b * b;
        let im = 1. / (k as f64 * fn_).cos();
        l.add_conjugates(c(a / d2, b / d2), c(0., im));
        k += 2;
    }
    if n & 1 == 1 {
        l.add(C::real(1. / sinh_v0), INFINITY);
    }
    l
}

/// Complete elliptic integral of the first kind (AGM).
fn elliptic_k(k: f64) -> f64 {
    let m = k * k;
    let (mut a, mut b) = (1f64, (1. - m).sqrt());
    let mut cc = a - b;
    loop {
        let co = cc;
        cc = (a - b) / 2.;
        let ao = (a + b) / 2.;
        b = (a * b).sqrt();
        a = ao;
        if cc >= co {
            break;
        }
    }
    PI / (a + a)
}
/// The Elliptic prototype's scratch arrays (upstream `double[100]`, widened so
/// order 50 stays in bounds).
struct Elliptic {
    zeros: Vec<f64>,
    c1: Vec<f64>,
    b1: Vec<f64>,
    a1: Vec<f64>,
    d1: Vec<f64>,
    q1: Vec<f64>,
    z1: Vec<f64>,
    s1: Vec<f64>,
    p: Vec<f64>,
    zw1: Vec<f64>,
    zf1: Vec<f64>,
    zq1: Vec<f64>,
    k: f64,
    kprime: f64,
    e: f64,
    nin: usize,
    n2: usize,
    m: usize,
    em: usize,
}
impl Elliptic {
    fn prodpoly(&mut self, sn: usize) {
        self.b1[0] = self.s1[1];
        self.b1[1] = 1.;
        for j in 2..=sn {
            self.a1[0] = self.s1[j] * self.b1[0];
            for i in 1..j {
                self.a1[i] = self.b1[i - 1] + self.s1[j] * self.b1[i];
            }
            for i in 0..j {
                self.b1[i] = self.a1[i];
            }
            self.b1[j] = 1.;
        }
    }
    fn calcfz2(&mut self, i: usize) {
        let (mut ji, mut jf) = (0, 0);
        if i < self.em + 2 {
            ji = 0;
            jf = i;
        }
        if i > self.em {
            ji = i - self.em;
            jf = self.em;
        }
        self.c1[i] = 0.;
        let mut j = ji;
        while j <= jf {
            self.c1[i] += self.a1[j] * (self.a1[i - j] * 10f64.powf(self.m as f64 - (i / 2) as f64));
            j += 2;
        }
    }
    fn calcfz(&mut self) {
        let mut i = 1;
        if self.nin == 1 {
            self.s1[i] = 1.;
            i += 1;
        }
        while i <= self.nin + self.n2 {
            self.s1[i] = self.z1[i - self.nin];
            self.s1[i + self.n2] = self.z1[i - self.nin];
            i += 1;
        }
        self.prodpoly(self.nin + 2 * self.n2);
        let mut i = 0;
        while i <= self.em {
            self.a1[i] = self.e * self.b1[i];
            i += 2;
        }
        let mut i = 0;
        while i <= 2 * self.em {
            self.calcfz2(i);
            i += 2;
        }
    }
    fn calcqz(&mut self) {
        let mut i = 1;
        while i <= self.nin {
            self.s1[i] = -10.;
            i += 1;
        }
        while i <= self.nin + self.n2 {
            self.s1[i] = -10. * self.z1[i - self.nin] * self.z1[i - self.nin];
            i += 1;
        }
        while i <= self.nin + 2 * self.n2 {
            self.s1[i] = self.s1[i - self.n2];
            i += 1;
        }
        self.prodpoly(self.m);
        let dd = if self.nin & 1 == 1 { -1. } else { 1. };
        let mut i = 0;
        while i <= 2 * self.m {
            self.d1[i] = dd * self.b1[i / 2];
            i += 2;
        }
    }
    fn findfact(&mut self, mut t: usize) -> f64 {
        let mut a = 0.;
        for i in 1..=t {
            self.a1[i] /= self.a1[0];
        }
        self.a1[0] = 1.;
        self.b1[0] = 1.;
        self.c1[0] = 1.;
        let mut i1 = 0;
        loop {
            if t <= 2 {
                break;
            }
            let (mut p0, mut q0) = (0f64, 0f64);
            i1 += 1;
            for _ in 0..100_000 {
                self.b1[1] = self.a1[1] - p0;
                self.c1[1] = self.b1[1] - p0;
                for i in 2..=t {
                    self.b1[i] = self.a1[i] - p0 * self.b1[i - 1] - q0 * self.b1[i - 2];
                }
                for i in 2..t {
                    self.c1[i] = self.b1[i] - p0 * self.c1[i - 1] - q0 * self.c1[i - 2];
                }
                let (x1, x2, x3) = (t - 1, t - 2, t - 3);
                let mut x4 = self.c1[x2] * self.c1[x2] + self.c1[x3] * (self.b1[x1] - self.c1[x1]);
                if x4 == 0. {
                    x4 = 1e-3;
                }
                let ddp = (self.b1[x1] * self.c1[x2] - self.b1[t] * self.c1[x3]) / x4;
                p0 += ddp;
                let dq = (self.b1[t] * self.c1[x2] - self.b1[x1] * (self.c1[x1] - self.b1[x1])) / x4;
                q0 += dq;
                if (ddp + dq).abs() < 1e-6 {
                    break;
                }
            }
            self.p[i1] = p0;
            self.q1[i1] = q0;
            self.a1[1] -= p0;
            t -= 2;
            for i in 2..=t {
                self.a1[i] -= p0 * self.a1[i - 1] + q0 * self.a1[i - 2];
            }
            if t <= 2 {
                break;
            }
        }
        if t == 2 {
            i1 += 1;
            self.p[i1] = self.a1[1];
            self.q1[i1] = self.a1[2];
        }
        if t == 1 {
            a = -self.a1[1];
        }
        a
    }
    fn calcsn(&self, u: f64) -> f64 {
        let mut sn = 0.;
        let q = (-PI * self.kprime / self.k).exp();
        let v = PI * 0.5 * u / self.k;
        let mut j = 0;
        loop {
            let w = q.powf(j as f64 + 0.5);
            sn += w * ((2 * j + 1) as f64 * v).sin() / (1. - w * w);
            if w < 1e-7 || j > 10_000 {
                break;
            }
            j += 1;
        }
        sn
    }
}
fn elliptic_lowpass(n: usize, ripple_db: f64, rolloff: f64) -> Layout {
    let size = 4 * n + 8;
    let v = || vec![0f64; size];
    let mut s = Elliptic {
        zeros: v(), c1: v(), b1: v(), a1: v(), d1: v(), q1: v(), z1: v(), s1: v(), p: v(),
        zw1: v(), zf1: v(), zq1: v(), k: 0., kprime: 0., e: 0., nin: 0, n2: 0, m: 0, em: 0,
    };
    let mut l = Layout::default();
    let e2 = 10f64.powf(ripple_db / 10.) - 1.;
    let xi = 5. * (rolloff - 1.).exp() + 1.;
    s.k = elliptic_k(1. / xi);
    s.kprime = elliptic_k((1. - 1. / (xi * xi)).sqrt());
    let ni = if n & 1 == 1 { 0 } else { 1 };
    let mut f = vec![0f64; size];
    for i in 1..=n / 2 {
        let u = (2 * i - ni) as f64 * s.k / n as f64;
        let sn = s.calcsn(u) * 2. * PI / s.k;
        s.zeros[i - 1] = 1. / sn;
        f[i] = 1. / sn;
    }
    s.zeros[n / 2] = f64::INFINITY;
    let fb = 1. / (2. * PI);
    s.nin = n % 2;
    s.n2 = n / 2;
    for i in 1..=s.n2 {
        let x = f[s.n2 + 1 - i];
        s.z1[i] = (1. - 1. / (x * x)).sqrt();
    }
    s.e = e2.sqrt();
    let fbb = fb * fb;
    s.m = s.nin + 2 * s.n2;
    s.em = 2 * (s.m / 2);
    let tp = 2. * PI;
    s.calcfz();
    s.calcqz();
    if s.m > s.em {
        s.c1[2 * s.m] = 0.;
    }
    let mut i = 0;
    while i <= 2 * s.m {
        s.a1[s.m - i / 2] = s.c1[i] + s.d1[i];
        i += 2;
    }
    let a0 = s.findfact(s.m);
    let mut r = 0;
    while r < s.em / 2 {
        r += 1;
        s.p[r] /= 10.;
        s.q1[r] /= 100.;
        let d = 1. + s.p[r] + s.q1[r];
        let b1 = (1. + s.p[r] / 2.) * fbb / d;
        s.zf1[r] = fb / d.powf(0.25);
        s.zq1[r] = 1. / (2. * (1. - b1 / (s.zf1[r] * s.zf1[r]))).abs().sqrt();
        s.zw1[r] = tp * s.zf1[r];
        let pole = c(
            -0.5 * s.zw1[r] / s.zq1[r],
            0.5 * (s.zw1[r] * s.zw1[r] / (s.zq1[r] * s.zq1[r]) - 4. * s.zw1[r] * s.zw1[r]).abs().sqrt(),
        );
        l.add_conjugates(pole, c(0., s.zeros[r - 1]));
    }
    if a0 != 0. {
        l.add(C::real(-(fbb / (0.1 * a0 - 1.)).sqrt() * tp), INFINITY);
    }
    l.set_normal(0., if n & 1 == 1 { 1. } else { 10f64.powf(-ripple_db / 20.) });
    l
}

/// Laguerre root finder with deflation, polishing and descending-imaginary sort.
struct Roots {
    a: Vec<C>,
    root: Vec<C>,
}
impl Roots {
    fn new(degree: usize) -> Self {
        Self { a: vec![C::default(); degree + 1], root: vec![C::default(); degree] }
    }
    fn laguerre(degree: usize, a: &[C], x: &mut C) {
        const MR: usize = 8;
        const MT: usize = 10;
        const FRAC: [f64; MR + 1] = [0.0, 0.5, 0.25, 0.75, 0.13, 0.38, 0.62, 0.88, 1.0];
        let m = degree;
        for iter in 1..=MT * MR {
            let mut b = a[m];
            let mut err = b.abs();
            let (mut d, mut f) = (C::default(), C::default());
            let abx = x.abs();
            for j in (0..m).rev() {
                f = *x * f + d;
                d = *x * d + b;
                b = *x * b + a[j];
                err = b.abs() + abx * err;
            }
            err *= f64::EPSILON;
            if b.abs() <= err {
                return;
            }
            let g = d / b;
            let g2 = g * g;
            let h = g2 - (f / b).scale(2.);
            let sq = ((h.scale(m as f64) - g2).scale((m - 1) as f64)).sqrt();
            let mut gp = g + sq;
            let gm = g - sq;
            let (abp, abm) = (gp.abs(), gm.abs());
            if abp < abm {
                gp = gm;
            }
            let dx = if abp.max(abm) > 0. { C::real(m as f64) / gp } else { C::polar(1. + abx, iter as f64) };
            let x1 = *x - dx;
            if *x == x1 {
                return;
            }
            if iter % MT != 0 {
                *x = x1;
            } else {
                *x = *x - dx.scale(FRAC[iter / MT]);
            }
        }
    }
    fn solve(&mut self, degree: usize) {
        const EPS: f64 = 1.0e-30;
        let mut ad = self.a[..=degree].to_vec();
        for j in (0..degree).rev() {
            let mut x = C::default();
            Self::laguerre(j + 1, &ad, &mut x);
            if x.im.abs() <= 2. * EPS * x.re.abs() {
                x = C::real(x.re);
            }
            self.root[j] = x;
            let mut b = ad[j + 1];
            for jj in (0..=j).rev() {
                let cc = ad[jj];
                ad[jj] = b;
                b = x * b + cc;
            }
        }
        for j in 0..degree {
            let mut r = self.root[j];
            Self::laguerre(degree, &self.a, &mut r);
            self.root[j] = r;
        }
        self.sort(degree);
    }
    fn sort(&mut self, degree: usize) {
        for j in 1..degree {
            let x = self.root[j];
            let mut i = j as isize - 1;
            while i >= 0 {
                if self.root[i as usize].im >= x.im {
                    break;
                }
                self.root[i as usize + 1] = self.root[i as usize];
                i -= 1;
            }
            self.root[(i + 1) as usize] = x;
        }
    }
}
fn fact(n: usize) -> f64 {
    (1..=n).fold(1., |y, m| y * m as f64)
}
fn reverse_bessel(k: usize, n: usize) -> f64 {
    fact(2 * n - k) / ((fact(n - k) * fact(k)) * 2f64.powf((n - k) as f64))
}
fn bessel_lowpass(n: usize) -> Layout {
    let mut l = Layout::default();
    l.set_normal(0., 1.);
    let mut s = Roots::new(n);
    for i in 0..=n {
        s.a[i] = C::real(reverse_bessel(i, n));
    }
    s.solve(n);
    for i in 0..n / 2 {
        l.add_conjugates(s.root[i], INFINITY);
    }
    if n & 1 == 1 {
        l.add(C::real(s.root[n / 2].re), INFINITY);
    }
    l
}
fn bessel_lowshelf(n: usize, gain_db: f64) -> Layout {
    let mut l = Layout::default();
    l.set_normal(PI, 1.);
    let g = 10f64.powf(gain_db / 20.) - 1.;
    let mut poles = Roots::new(n);
    let mut zeros = Roots::new(n);
    for i in 0..=n {
        poles.a[i] = C::real(reverse_bessel(i, n));
        zeros.a[i] = C::real(reverse_bessel(i, n));
    }
    poles.solve(n);
    zeros.a[0] = zeros.a[0] + C::real(g * reverse_bessel(0, n));
    zeros.solve(n);
    for i in 0..n / 2 {
        l.add_conjugates(poles.root[i], zeros.root[i]);
    }
    if n & 1 == 1 {
        l.add(C::real(poles.root[n / 2].re), C::real(zeros.root[n / 2].re));
    }
    l
}
/// Coefficients of the Legendre polynomial of the first kind, degree `n`.
fn legendre(p: &mut [f64], n: usize, aa: &mut [f64], bb: &mut [f64]) {
    if n == 0 {
        p[0] = 1.;
        return;
    }
    if n == 1 {
        p[0] = 0.;
        p[1] = 1.;
        return;
    }
    p[0] = -0.5;
    p[1] = 0.;
    p[2] = 1.5;
    if n == 2 {
        return;
    }
    for i in 0..=n {
        aa[i] = 0.;
        bb[i] = 0.;
    }
    bb[1] = 1.;
    for i in 3..=n {
        for j in 0..=i {
            aa[j] = bb[j];
            bb[j] = p[j];
            p[j] = 0.;
        }
        let mut j = i as isize - 2;
        while j >= 0 {
            p[j as usize] -= (i - 1) as f64 * aa[j as usize] / i as f64;
            j -= 2;
        }
        let mut j = i as isize - 1;
        while j >= 0 {
            p[j as usize + 1] += (2 * i - 1) as f64 * bb[j as usize] / i as f64;
            j -= 2;
        }
    }
}
/// Papoulis optimum-L polynomial (C. Bond 2004): `w[]` of degree `n`.
fn legendre_polynomial(n: usize) -> Vec<f64> {
    let size = 2 * n + 8;
    let (mut a, mut s, mut w, mut v, mut p) = (vec![0.; size], vec![0.; size], vec![0.; size], vec![0.; size], vec![0.; size]);
    let (mut aa, mut bb) = (vec![0.; size], vec![0.; size]);
    let k = (n - 1) / 2;
    let sqrt2 = std::f64::consts::SQRT_2;
    if n & 1 == 1 {
        for i in 0..=k {
            a[i] = (2. * i as f64 + 1.) / (sqrt2 * (k as f64 + 1.));
        }
    } else {
        for i in 0..k + 1 {
            a[i] = 0.;
        }
        let start = if k & 1 == 1 { 1 } else { 0 };
        let mut i = start;
        while i <= k {
            a[i] = (2 * i + 1) as f64 / (((k + 1) * (k + 2)) as f64).sqrt();
            i += 2;
        }
    }
    for i in 0..=n {
        s[i] = 0.;
        w[i] = 0.;
    }
    s[0] = a[0];
    s[1] = a[1];
    for i in 2..=k {
        legendre(&mut p, i, &mut aa, &mut bb);
        for j in 0..=i {
            s[j] += a[i] * p[j];
        }
    }
    for i in 0..=2 * k + 2 {
        v[i] = 0.;
    }
    for i in 0..=k {
        for j in 0..=k {
            v[i + j] += s[i] * s[j];
        }
    }
    v[2 * k + 1] = 0.;
    if n & 1 == 0 {
        for i in (0..=n).rev() {
            v[i + 1] += v[i];
        }
    }
    for i in (0..=n + 1).rev() {
        v[i + 1] = v[i] / (i as f64 + 1.);
    }
    v[0] = 0.;
    for i in 0..n + 2 {
        s[i] = 0.;
    }
    s[0] = -1.;
    s[1] = 2.;
    for i in 1..=n {
        if i > 1 {
            let mut c0 = -s[0];
            for j in 1..i + 1 {
                let c1 = -s[j] + 2. * s[j - 1];
                s[j - 1] = c0;
                c0 = c1;
            }
            let c1 = 2. * s[i];
            s[i] = c0;
            s[i + 1] = c1;
        }
        for j in (1..=i).rev() {
            w[j] += v[i] * s[j];
        }
    }
    if n & 1 == 0 {
        w[1] = 0.;
    }
    w
}
fn legendre_lowpass(n: usize) -> Layout {
    let mut l = Layout::default();
    l.set_normal(0., 1.);
    let w = legendre_polynomial(n);
    let degree = n * 2;
    let mut poles = Roots::new(degree);
    poles.a[0] = C::real(1. + w[0]);
    poles.a[1] = C::default();
    for i in 1..=n {
        poles.a[2 * i] = C::real(w[i] * if i & 1 == 1 { -1. } else { 1. });
        if 2 * i + 1 <= degree {
            poles.a[2 * i + 1] = C::default();
        }
    }
    poles.solve(degree);
    let mut j = 0;
    for i in 0..degree {
        if poles.root[i].re <= 0. {
            poles.root[j] = poles.root[i];
            j += 1;
        }
    }
    poles.sort(degree / 2);
    for i in 0..n / 2 {
        l.add_conjugates(poles.root[i], INFINITY);
    }
    if n & 1 == 1 {
        l.add(C::real(poles.root[n / 2].re), INFINITY);
    }
    l
}

// ---- digital transforms --------------------------------------------------

fn lowpass_transform(fc: f64, analog: &Layout) -> Layout {
    let mut d = Layout::default();
    let f = (PI * fc).tan();
    let t = |c: C| if c == INFINITY { C::real(-1.) } else { let c = c.scale(f); (C::real(1.) + c) / (C::real(1.) - c) };
    for pair in &analog.pairs[..analog.poles / 2] {
        d.add_conjugates(t(pair.p1), t(pair.z1));
    }
    if analog.poles & 1 == 1 {
        let pair = analog.pairs[analog.poles / 2];
        d.add(t(pair.p1), t(pair.z1));
    }
    d.set_normal(analog.normal_w, analog.normal_gain);
    d
}
fn highpass_transform(fc: f64, analog: &Layout) -> Layout {
    let mut d = Layout::default();
    let f = 1. / (PI * fc).tan();
    let t = |c: C| if c == INFINITY { C::real(1.) } else { let c = c.scale(f); -((C::real(1.) + c) / (C::real(1.) - c)) };
    for pair in &analog.pairs[..analog.poles / 2] {
        d.add_conjugates(t(pair.p1), t(pair.z1));
    }
    if analog.poles & 1 == 1 {
        let pair = analog.pairs[analog.poles / 2];
        d.add(t(pair.p1), t(pair.z1));
    }
    d.set_normal(PI - analog.normal_w, analog.normal_gain);
    d
}
fn band_edges(fc: f64, fw: f64) -> (f64, f64) {
    let ww = 2. * PI * fw;
    let mut wc2 = 2. * PI * fc - ww / 2.;
    let mut wc = wc2 + ww;
    if wc2 < 1e-8 {
        wc2 = 1e-8;
    }
    if wc > PI - 1e-8 {
        wc = PI - 1e-8;
    }
    (wc, wc2)
}
fn bandpass_transform(fc: f64, fw: f64, analog: &Layout) -> Layout {
    let mut d = Layout::default();
    let (wc, wc2) = band_edges(fc, fw);
    let a = ((wc + wc2) * 0.5).cos() / ((wc - wc2) * 0.5).cos();
    let b = 1. / ((wc - wc2) * 0.5).tan();
    let (a2, b2) = (a * a, b * b);
    let ab_2 = 2. * a * b;
    let t = |c: C| -> (C, C) {
        if c == INFINITY {
            return (C::real(-1.), C::real(1.));
        }
        let c = (C::real(1.) + c) / (C::real(1.) - c);
        let mut v = addmul(C::default(), 4. * (b2 * (a2 - 1.) + 1.), c);
        v = v + C::real(8. * (b2 * (a2 - 1.) - 1.));
        v = v * c;
        v = v + C::real(4. * (b2 * (a2 - 1.) + 1.));
        v = v.sqrt();
        let mut u = -v;
        u = addmul(u, ab_2, c) + C::real(ab_2);
        v = addmul(v, ab_2, c) + C::real(ab_2);
        let dd = addmul(C::default(), 2. * (b - 1.), c) + C::real(2. * (1. + b));
        (u / dd, v / dd)
    };
    for pair in &analog.pairs[..analog.poles / 2] {
        let p = t(pair.p1);
        let z = t(pair.z1);
        d.add_conjugates(p.0, z.0);
        d.add_conjugates(p.1, z.1);
    }
    if analog.poles & 1 == 1 {
        let pair = analog.pairs[analog.poles / 2];
        d.add_pairs(t(pair.p1), t(pair.z1));
    }
    let wn = analog.normal_w;
    d.set_normal(2. * (((wc + wn) * 0.5).tan() * ((wc2 + wn) * 0.5).tan()).sqrt().atan(), analog.normal_gain);
    d
}
fn bandstop_transform(fc: f64, fw: f64, analog: &Layout) -> Layout {
    let mut d = Layout::default();
    let (wc, wc2) = band_edges(fc, fw);
    let a = ((wc + wc2) * 0.5).cos() / ((wc - wc2) * 0.5).cos();
    let b = ((wc - wc2) * 0.5).tan();
    let (a2, b2) = (a * a, b * b);
    let t = |c: C| -> (C, C) {
        let c = if c == INFINITY { C::real(-1.) } else { (C::real(1.) + c) / (C::real(1.) - c) };
        let mut u = addmul(C::default(), 4. * (b2 + a2 - 1.), c);
        u = u + C::real(8. * (b2 - a2 + 1.));
        u = u * c;
        u = u + C::real(4. * (a2 + b2 - 1.));
        u = u.sqrt();
        let mut v = u.scale(-0.5);
        v = v + C::real(a);
        v = addmul(v, -a, c);
        u = u.scale(0.5);
        u = u + C::real(a);
        u = addmul(u, -a, c);
        let dd = addmul(C::real(b + 1.), b - 1., c);
        (u / dd, v / dd)
    };
    for pair in &analog.pairs[..analog.poles / 2] {
        let p = t(pair.p1);
        let mut z = t(pair.z1);
        // Release-build conjugate shortcut of the upstream transform.
        if z.1 == z.0 {
            z.1 = z.0.conj();
        }
        d.add_conjugates(p.0, z.0);
        d.add_conjugates(p.1, z.1);
    }
    if analog.poles & 1 == 1 {
        let pair = analog.pairs[analog.poles / 2];
        d.add_pairs(t(pair.p1), t(pair.z1));
    }
    d.set_normal(if fc < 0.25 { PI } else { 0. }, analog.normal_gain);
    d
}

// ---- biquads ------------------------------------------------------------

/// One normalised second-order section (`a0` kept, others divided by it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Biquad {
    a0: f64,
    a1: f64,
    a2: f64,
    b0: f64,
    b1: f64,
    b2: f64,
}
impl Biquad {
    fn new(a0: f64, a1: f64, a2: f64, b0: f64, b1: f64, b2: f64) -> Self {
        Self { a0, a1: a1 / a0, a2: a2 / a0, b0: b0 / a0, b1: b1 / a0, b2: b2 / a0 }
    }
    fn one_pole(pole: C, zero: C) -> Self {
        Self::new(1., -pole.re, 0., -zero.re, 1., 0.)
    }
    fn two_pole(p1: C, z1: C, p2: C, z2: C) -> Self {
        let (a1, a2) = if p1.im != 0. { (-2. * p1.re, p1.norm()) } else { (-(p1.re + p2.re), p1.re * p2.re) };
        let (b1, b2) = if z1.im != 0. { (-2. * z1.re, z1.norm()) } else { (-(z1.re + z2.re), z1.re * z2.re) };
        Self::new(1., a1, a2, 1., b1, b2)
    }
    fn scale(&mut self, k: f64) {
        self.b0 *= k;
        self.b1 *= k;
        self.b2 *= k;
    }
}
fn cascade(layout: &Layout) -> Vec<Biquad> {
    let mut stages: Vec<Biquad> = layout.pairs[..layout.poles.div_ceil(2)]
        .iter()
        .map(|p| if p.single() { Biquad::one_pole(p.p1, p.z1) } else { Biquad::two_pole(p.p1, p.z1, p.p2, p.z2) })
        .collect();
    let w = layout.normal_w;
    let (czn1, czn2) = (C::polar(1., -w), C::polar(1., -2. * w));
    let (mut ch, mut cbot) = (C::real(1.), C::real(1.));
    for s in &stages {
        // getB*()/getA0() as upstream: the stored values times a0 over a0.
        let ct = addmul(addmul(C::real(s.b0 * s.a0 / s.a0), s.b1 * s.a0 / s.a0, czn1), s.b2 * s.a0 / s.a0, czn2);
        let cb = addmul(addmul(C::real(1.), s.a1 * s.a0 / s.a0, czn1), s.a2 * s.a0 / s.a0, czn2);
        ch = ch * ct;
        cbot = cbot * cb;
    }
    let gain = layout.normal_gain / (ch / cbot).abs();
    if let Some(first) = stages.first_mut() {
        first.scale(gain);
    }
    stages
}

// ---- parameters ---------------------------------------------------------

pub use crysirror_core::service::{IirParamInfo as ParamInfo, iir_default_params as default_params, iir_max_order as max_order, iir_params as params};

/// The biquad cascade for `params` (DSPFilters `setParams`/`setup` order;
/// the pole order is truncated and clamped to the family's range).
pub fn design(kind: IirKind, family: IirFamily, p: &[f64; 8]) -> Option<Vec<Biquad>> {
    use IirFamily as F;
    use IirKind as K;
    params(kind, family)?;
    let sr = p[0];
    if family == F::Rbj || family == F::Custom {
        let w0 = 2. * PI * p[1] / sr;
        let (cs, sn) = (w0.cos(), w0.sin());
        let q = p[2];
        let bq = match kind {
            K::LowPass => {
                let al = sn / (2. * q);
                Biquad::new(1. + al, -2. * cs, 1. - al, (1. - cs) / 2., 1. - cs, (1. - cs) / 2.)
            }
            K::HighPass => {
                let al = sn / (2. * q);
                Biquad::new(1. + al, -2. * cs, 1. - al, (1. + cs) / 2., -(1. + cs), (1. + cs) / 2.)
            }
            K::BandPass => {
                let al = sn / (2. * q);
                Biquad::new(1. + al, -2. * cs, 1. - al, q * al, 0., -q * al)
            }
            K::BandPass2 => {
                let al = sn / (2. * q);
                Biquad::new(1. + al, -2. * cs, 1. - al, al, 0., -al)
            }
            K::BandStop => {
                let al = sn / (2. * q);
                Biquad::new(1. + al, -2. * cs, 1. - al, 1., -2. * cs, 1.)
            }
            K::AllPass => {
                let al = sn / (2. * q);
                Biquad::new(1. + al, -2. * cs, 1. - al, 1. - al, -2. * cs, 1. + al)
            }
            K::LowShelf | K::HighShelf => {
                let a = 10f64.powf(p[2] / 40.);
                let al = sn / 2. * ((a + 1. / a) * (1. / p[3] - 1.) + 2.).sqrt();
                let sq = 2. * a.sqrt() * al;
                if kind == K::LowShelf {
                    Biquad::new(
                        (a + 1.) + (a - 1.) * cs + sq,
                        -2. * ((a - 1.) + (a + 1.) * cs),
                        (a + 1.) + (a - 1.) * cs - sq,
                        a * ((a + 1.) - (a - 1.) * cs + sq),
                        2. * a * ((a - 1.) - (a + 1.) * cs),
                        a * ((a + 1.) - (a - 1.) * cs - sq),
                    )
                } else {
                    Biquad::new(
                        (a + 1.) - (a - 1.) * cs + sq,
                        2. * ((a - 1.) - (a + 1.) * cs),
                        (a + 1.) - (a - 1.) * cs - sq,
                        a * ((a + 1.) + (a - 1.) * cs + sq),
                        -2. * a * ((a - 1.) + (a + 1.) * cs),
                        a * ((a + 1.) + (a - 1.) * cs - sq),
                    )
                }
            }
            K::BandShelf => {
                let a = 10f64.powf(p[2] / 40.);
                let al = sn * (std::f64::consts::LN_2 / 2. * p[3] * w0 / sn).sinh();
                Biquad::new(1. + al / a, -2. * cs, 1. - al / a, 1. + al * a, -2. * cs, 1. - al * a)
            }
            K::OnePole => {
                let mut b = Biquad::one_pole(C::real(p[2]), C::real(p[3]));
                b.scale(10f64.powf(p[1] / 20.));
                b
            }
            K::TwoPole => {
                let pole = C::polar(p[2], p[3]);
                let zero = C::polar(p[4], p[5]);
                let mut b = Biquad::two_pole(pole, zero, pole.conj(), zero.conj());
                b.scale(10f64.powf(p[1] / 20.));
                b
            }
        };
        return Some(vec![bq]);
    }
    let order = (p[1] as i64).clamp(1, max_order(family) as i64) as usize;
    let (extra0, extra1) = match kind {
        K::BandPass | K::BandStop | K::BandShelf => (p[4], p[5]),
        _ => (p[3], p[4]),
    };
    let shelf_gain = match kind {
        K::BandShelf => p[4],
        _ => p[3],
    };
    let shelf_extra = match kind {
        K::BandShelf => p[5],
        _ => p[4],
    };
    let analog = match (family, kind) {
        (F::Butterworth, K::LowShelf | K::HighShelf | K::BandShelf) => butterworth_lowshelf(order, shelf_gain),
        (F::Butterworth, _) => butterworth_lowpass(order),
        (F::ChebyshevI, K::LowShelf | K::HighShelf | K::BandShelf) => chebyshev_lowshelf(order, shelf_gain, shelf_extra),
        (F::ChebyshevI, _) => chebyshev1_lowpass(order, extra0),
        (F::ChebyshevII, K::LowShelf | K::HighShelf | K::BandShelf) => chebyshev_lowshelf(order, shelf_gain, shelf_extra),
        (F::ChebyshevII, _) => chebyshev2_lowpass(order, extra0),
        (F::Elliptic, _) => elliptic_lowpass(order, extra0, extra1),
        (F::Bessel, K::LowShelf) => bessel_lowshelf(order, shelf_gain),
        (F::Bessel, _) => bessel_lowpass(order),
        (F::Legendre, _) => legendre_lowpass(order),
        _ => return None,
    };
    let fc = p[2] / sr;
    let mut digital = match kind {
        K::LowPass | K::LowShelf => lowpass_transform(fc, &analog),
        K::HighPass | K::HighShelf => highpass_transform(fc, &analog),
        K::BandPass | K::BandShelf => bandpass_transform(fc, p[3] / sr, &analog),
        K::BandStop => bandstop_transform(fc, p[3] / sr, &analog),
        _ => return None,
    };
    if kind == K::BandShelf {
        digital.set_normal(if fc < 0.25 { PI } else { 0. }, 1.);
    }
    Some(cascade(&digital))
}

/// Per-channel cascade state in one of the four upstream forms.
#[derive(Clone, Debug)]
pub struct State {
    structure: IirStructure,
    s: Vec<[f64; 9]>,
    vsa: f64,
}
impl State {
    pub fn new(structure: IirStructure, stages: usize) -> Self {
        Self { structure, s: vec![[0.; 9]; stages], vsa: 1e-8 }
    }
    /// `Cascade::State::process`: the anti-denormal term alternates sign
    /// every sample and only enters the first stage.
    pub fn process(&mut self, input: f64, stages: &[Biquad]) -> f64 {
        if self.s.len() < stages.len() {
            self.s.resize(stages.len(), [0.; 9]);
        }
        self.vsa = -self.vsa;
        let mut out = input;
        for (i, (st, b)) in self.s.iter_mut().zip(stages).enumerate() {
            let vsa = if i == 0 { self.vsa } else { 0. };
            out = match self.structure {
                IirStructure::DirectFormI => {
                    let [x1, x2, y1, y2, ..] = *st;
                    let y = b.b0 * out + b.b1 * x1 + b.b2 * x2 - b.a1 * y1 - b.a2 * y2 + vsa;
                    st[1] = x1;
                    st[3] = y1;
                    st[0] = out;
                    st[2] = y;
                    y
                }
                IirStructure::DirectFormII => {
                    let [v1, v2, ..] = *st;
                    let w = out - b.a1 * v1 - b.a2 * v2 + vsa;
                    let y = b.b0 * w + b.b1 * v1 + b.b2 * v2;
                    st[1] = v1;
                    st[0] = w;
                    y
                }
                IirStructure::TransposedDirectFormI => {
                    // [v, s1, s1_1, s2, s2_1, s3, s3_1, s4, s4_1]
                    let v = out + st[2];
                    let y = b.b0 * v + st[6];
                    st[0] = v;
                    st[1] = st[4] - b.a1 * v;
                    st[3] = -b.a2 * v;
                    st[5] = b.b1 * v + st[8];
                    st[7] = b.b2 * v;
                    st[8] = st[7];
                    st[6] = st[5];
                    st[4] = st[3];
                    st[2] = st[1];
                    y
                }
                IirStructure::TransposedDirectFormII => {
                    // [s1, s1_1, s2, s2_1]
                    let y = st[1] + b.b0 * out + vsa;
                    st[0] = st[3] + b.b1 * out - b.a1 * y;
                    st[2] = b.b2 * out - b.a2 * y;
                    st[1] = st[0];
                    st[3] = st[2];
                    y
                }
            };
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn gain_at(stages: &[Biquad], f: f64, sr: f64) -> f64 {
        let w = 2. * PI * f / sr;
        let (z1, z2) = (C::polar(1., -w), C::polar(1., -2. * w));
        let (mut n, mut d) = (C::real(1.), C::real(1.));
        for s in stages {
            n = n * addmul(addmul(C::real(s.b0), s.b1, z1), s.b2, z2);
            d = d * addmul(addmul(C::real(1.), s.a1, z1), s.a2, z2);
        }
        (n / d).abs()
    }
    fn p(v: &[f64]) -> [f64; 8] {
        let mut o = [0.; 8];
        o[..v.len()].copy_from_slice(v);
        o
    }
    #[test]
    fn pole_families_meet_their_magnitude_specs() {
        let sr = 48000.;
        for (fam, extra) in [
            (IirFamily::Butterworth, vec![]),
            (IirFamily::ChebyshevI, vec![1.]),
            (IirFamily::ChebyshevII, vec![40.]),
            (IirFamily::Elliptic, vec![1., 0.]),
            (IirFamily::Bessel, vec![]),
            (IirFamily::Legendre, vec![]),
        ] {
            for order in [1, 2, 3, 4, 7, 8] {
                let mut v = vec![sr, order as f64, 1000.];
                v.extend(&extra);
                let lp = design(IirKind::LowPass, fam, &p(&v)).unwrap();
                assert_eq!(lp.len(), (order + 1) / 2, "{fam:?} {order}");
                let dc = gain_at(&lp, 0., sr);
                let stop = gain_at(&lp, 16000., sr);
                assert!(dc > 0.85 && dc < 1.0001, "{fam:?} {order} dc {dc}");
                assert!(stop < dc, "{fam:?} {order} stop {stop}");
                let hp = design(IirKind::HighPass, fam, &p(&v)).unwrap();
                assert!(gain_at(&hp, 20., sr) < gain_at(&hp, 20000., sr), "{fam:?} {order} hp");
            }
            let mut v = vec![sr, 4., 2000., 1000.];
            v.extend(&extra);
            let bp = design(IirKind::BandPass, fam, &p(&v)).unwrap();
            assert!(gain_at(&bp, 2000., sr) > 10. * gain_at(&bp, 100., sr), "{fam:?} bp");
            let bs = design(IirKind::BandStop, fam, &p(&v)).unwrap();
            assert!(gain_at(&bs, 2000., sr) < 0.1 * gain_at(&bs, 100., sr), "{fam:?} bs");
        }
        // Butterworth -3 dB at the cutoff.
        let lp = design(IirKind::LowPass, IirFamily::Butterworth, &p(&[sr, 4., 1000.])).unwrap();
        assert!((gain_at(&lp, 1000., sr) - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9);
        // Shelves reach their gain.
        let ls = design(IirKind::LowShelf, IirFamily::Butterworth, &p(&[sr, 3., 500., -12.])).unwrap();
        assert!((20. * gain_at(&ls, 1., sr).log10() + 12.).abs() < 0.05);
        assert!((gain_at(&ls, 20000., sr) - 1.).abs() < 0.01);
    }
    #[test]
    fn rbj_and_custom_match_the_cookbook() {
        let sr = 44100.;
        let lp = design(IirKind::LowPass, IirFamily::Rbj, &p(&[sr, 1000., FRAC_Q])).unwrap();
        assert!((gain_at(&lp, 1000., sr) - FRAC_Q).abs() < 1e-9);
        let ap = design(IirKind::AllPass, IirFamily::Rbj, &p(&[sr, 1000., 1.])).unwrap();
        for f in [10., 1000., 15000.] {
            assert!((gain_at(&ap, f, sr) - 1.).abs() < 1e-9);
        }
        let one = design(IirKind::OnePole, IirFamily::Custom, &p(&[sr, 0., 0.25, -0.25])).unwrap();
        assert_eq!(one[0], Biquad { a0: 1., a1: -0.25, a2: 0., b0: 0.25, b1: 1., b2: 0. });
        assert!(design(IirKind::LowShelf, IirFamily::Elliptic, &p(&[sr])).is_none());
        assert_eq!(default_params(IirKind::BandPass, IirFamily::Butterworth).unwrap()[..4], [44100., 2., 2000., 1720.]);
        let info = params(IirKind::LowPass, IirFamily::Rbj).unwrap()[2];
        assert_eq!((info.min, info.max), (0.0625, 16.));
    }
    const FRAC_Q: f64 = std::f64::consts::FRAC_1_SQRT_2;
    #[test]
    fn every_state_form_realises_the_same_filter() {
        let st = design(IirKind::LowPass, IirFamily::Butterworth, &p(&[48000., 4., 3000.])).unwrap();
        let input: Vec<f64> = (0..512).map(|i| ((i * 7919) % 97) as f64 / 48. - 1.).collect();
        let run = |s: IirStructure| {
            let mut state = State::new(s, st.len());
            input.iter().map(|x| state.process(*x, &st)).collect::<Vec<_>>()
        };
        let base = run(IirStructure::DirectFormI);
        for s in [IirStructure::DirectFormII, IirStructure::TransposedDirectFormI, IirStructure::TransposedDirectFormII] {
            let other = run(s);
            for (a, b) in base.iter().zip(&other) {
                assert!((a - b).abs() < 1e-6, "{s:?}");
            }
        }
    }
}
