use crate::model::TunnelTraffic;
use std::collections::{BTreeMap, VecDeque};

pub const WINDOW_SECONDS: i64 = 900;

#[derive(Clone, Copy, Debug, Default)]
pub struct Point {
    pub second: i64,
    pub bytes_in: u64,
    pub bytes_out: u64,
}

#[derive(Clone, Debug, Default)]
struct Track {
    previous: Option<(i64, TunnelTraffic)>,
    points: VecDeque<Point>,
}

#[derive(Clone, Debug, Default)]
pub struct History {
    tracks: BTreeMap<String, Track>,
}

impl History {
    pub fn has_points(&self) -> bool {
        self.tracks.values().any(|track| !track.points.is_empty())
    }

    pub fn record(&mut self, name: &str, traffic: &TunnelTraffic, second: i64) {
        let track = self.tracks.entry(name.to_owned()).or_default();
        let mut point = Point {
            second,
            ..Default::default()
        };
        if let Some((last, counters)) = &track.previous {
            if second <= *last {
                return;
            }
            let elapsed = u64::try_from(second - last).unwrap_or(1);
            point.bytes_in = traffic.bytes_in.saturating_sub(counters.bytes_in) / elapsed;
            point.bytes_out = traffic.bytes_out.saturating_sub(counters.bytes_out) / elapsed;
        }
        track.previous = Some((second, traffic.clone()));
        track.points.push_back(point);
        self.prune(second);
    }

    pub fn prune(&mut self, second: i64) {
        for track in self.tracks.values_mut() {
            while track
                .points
                .front()
                .is_some_and(|p| p.second <= second - WINDOW_SECONDS)
            {
                track.points.pop_front();
            }
        }
    }

    pub fn remove(&mut self, name: &str) {
        self.tracks.remove(name);
    }

    pub fn retained(&self, name: &str) -> usize {
        self.tracks.get(name).map_or(0, |track| track.points.len())
    }

    /// Zero-fill gaps and align all tunnels on the same one-second buckets before summing.
    pub fn series(&self, selected: Option<&str>, now: i64) -> Vec<Point> {
        let start = now - WINDOW_SECONDS + 1;
        let mut points: Vec<_> = (start..=now)
            .map(|second| Point {
                second,
                ..Default::default()
            })
            .collect();
        for (name, track) in &self.tracks {
            if selected.is_some_and(|selected| selected != name) {
                continue;
            }
            for sample in &track.points {
                if let Ok(index) = usize::try_from(sample.second - start)
                    && let Some(point) = points.get_mut(index)
                {
                    point.bytes_in = point.bytes_in.saturating_add(sample.bytes_in);
                    point.bytes_out = point.bytes_out.saturating_add(sample.bytes_out);
                }
            }
        }
        points
    }
}
