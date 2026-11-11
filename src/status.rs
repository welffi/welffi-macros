#[derive(Default)]
pub struct Status {
    pub step: String,
    pub runs: u32,
    pub wins: u32,
    pub losses: u32,
}

impl Status {
    pub fn record(&mut self, won: bool) {
        if won {
            self.wins += 1;
        } else {
            self.losses += 1;
        }
    }
}
