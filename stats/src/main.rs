use std::collections::{BTreeMap, HashMap};

use clap::Parser;
use espresso_types::SeqTypes;
use hotshot_task_impls::stats::{LeaderViewStats, ReplicaViewStats};
use hotshot_types::data::ViewNumber;
use plotly::{
    common::{HoverInfo, Line, Marker, MarkerSymbol, Mode},
    layout::{self, Axis, GridPattern, LayoutGrid},
    Bar, Layout, Plot, Scatter,
};
#[derive(Parser)]
struct Command {
    /// Path to the replica stats CSV file
    #[arg(long, default_value = "replica_stats.csv")]
    replica_path: String,

    /// Path to the leader stats CSV file
    #[arg(long, default_value = "leader_stats.csv")]
    leader_path: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::parse();

    let replica_view_stats = read_replica_view_stats(&command.replica_path)?;
    plot_replica_stats(&replica_view_stats)?;
    let stats = generate_replica_stats(&replica_view_stats);
    print_replica_stats(&stats);

    let leader_view_stats = read_leader_view_stats(&command.leader_path)?;
    plot_leader_stats(&leader_view_stats)?;

    Ok(())
}

struct ReplicaStats {
    vid_vs_dac: HashMap<String, usize>,
    proposal_vs_vid: HashMap<String, usize>,
    proposal_vs_dac: HashMap<String, usize>,
}

fn read_replica_view_stats(
    path: &str,
) -> Result<BTreeMap<ViewNumber, ReplicaViewStats<SeqTypes>>, Box<dyn std::error::Error>> {
    println!("\n**--- Replica Stats ---**");
    let mut reader = csv::Reader::from_path(path)?;
    let mut replica_view_stats = BTreeMap::new();

    for result in reader.deserialize() {
        let record: ReplicaViewStats<SeqTypes> = result?;
        replica_view_stats.insert(record.view, record);
    }

    Ok(replica_view_stats)
}

fn plot_replica_stats(
    replica_view_stats: &BTreeMap<ViewNumber, ReplicaViewStats<SeqTypes>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut x_views_normal = Vec::new();
    let mut y_timestamps_normal = Vec::new();
    let mut hover_texts_normal = Vec::new();

    let mut x_views_timeout = Vec::new();
    let mut y_timestamps_timeout = Vec::new();
    let mut hover_texts_timeout = Vec::new();

    let mut first_event_counts = HashMap::new();
    let mut views = Vec::new();
    let mut proposal_times = Vec::new();
    let mut vid_share_times = Vec::new();
    let mut dac_times = Vec::new();

    for (&view, record) in replica_view_stats {
        let proposal_ts = match record.proposal_recv {
            Some(t) => t,
            None => continue,
        };

        let mut events_with_ts = Vec::new();
        if let Some(ts) = record.proposal_recv {
            events_with_ts.push(("proposal_recv", ts / 1000));
        }
        if let Some(ts) = record.vote_send {
            events_with_ts.push(("vote_send", ts / 1000));
        }
        if let Some(ts) = record.timeout_vote_send {
            events_with_ts.push(("timeout_vote_send", ts / 1000));
        }
        if let Some(ts) = record.da_proposal_received {
            events_with_ts.push(("da_proposal_received", ts / 1000));
        }
        if let Some(ts) = record.da_proposal_validated {
            events_with_ts.push(("da_proposal_validated", ts / 1000));
        }
        if let Some(ts) = record.da_certificate_recv {
            events_with_ts.push(("da_certificate_recv", ts / 1000));
        }
        if let Some(ts) = record.proposal_prelim_validated {
            events_with_ts.push(("proposal_prelim_validated", ts / 1000));
        }
        if let Some(ts) = record.proposal_validated {
            events_with_ts.push(("proposal_validated", ts / 1000));
        }
        if let Some(ts) = record.timeout_triggered {
            events_with_ts.push(("timeout_triggered", ts / 1000));
        }
        if let Some(ts) = record.vid_share_validated {
            events_with_ts.push(("vid_share_validated", ts / 1000));
        }
        if let Some(ts) = record.vid_share_recv {
            events_with_ts.push(("vid_share_recv", ts / 1000));
        }

        if let Some((first_event, _)) = events_with_ts.clone().into_iter().min_by_key(|(_, ts)| *ts)
        {
            *first_event_counts.entry(first_event).or_insert(0) += 1;
        }

        events_with_ts.sort_by_key(|&(_, ts)| ts);
        let ordered_events = events_with_ts
            .iter()
            .enumerate()
            .map(|(i, (name, _))| format!("{}. {}", i + 1, name))
            .collect::<Vec<_>>()
            .join("<br>");
        let hover = format!("View: {view}<br>Events:<br>{ordered_events}");

        if record.timeout_triggered.is_some() {
            x_views_timeout.push(view);
            y_timestamps_timeout.push((proposal_ts as f64) / 1_000_000.0);
            hover_texts_timeout.push(hover);
        } else {
            x_views_normal.push(view);
            y_timestamps_normal.push((proposal_ts as f64) / 1_000_000.0);
            hover_texts_normal.push(hover);
        }

        views.push(view);
        proposal_times.push(record.proposal_recv.map(|t| t as f64));
        vid_share_times.push(record.vid_share_recv.map(|t| t as f64));
        dac_times.push(record.da_certificate_recv.map(|t| t as f64));
    }

    let mut first_events: Vec<_> = first_event_counts.into_iter().collect();
    first_events.sort_by(|a, b| b.1.cmp(&a.1));
    let (bar_labels, bar_values): (Vec<_>, Vec<_>) = first_events
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .unzip();

    let trace_normal = Scatter::new(x_views_normal, y_timestamps_normal)
        .mode(Mode::Markers)
        .hover_info(HoverInfo::Text)
        .hover_text_array(hover_texts_normal)
        .name("Proposal Received");

    let trace_timeout = Scatter::new(x_views_timeout, y_timestamps_timeout)
        .mode(Mode::Markers)
        .hover_info(HoverInfo::Text)
        .hover_text_array(hover_texts_timeout)
        .marker(Marker::new().color("red").symbol(MarkerSymbol::Circle))
        .name("Timeout Triggered");

    let trace_bar = Bar::new(bar_labels, bar_values)
        .name("First Event Frequency")
        .x_axis("x2")
        .y_axis("y2");

    let trace_proposal_time = Scatter::new(views.clone(), proposal_times)
        .mode(Mode::Markers)
        .name("Proposal Received")
        .x_axis("x3")
        .y_axis("y3")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Circle)
                .size(6)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("green").width(1.0)),
        );

    let trace_vid_share_time = Scatter::new(views.clone(), vid_share_times)
        .mode(Mode::Markers)
        .name("VID Share Received")
        .x_axis("x3")
        .y_axis("y3")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Square)
                .size(8)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("orange").width(1.0)),
        );

    let trace_dac_time = Scatter::new(views.clone(), dac_times)
        .mode(Mode::Markers)
        .name("DAC Received")
        .x_axis("x3")
        .y_axis("y3")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Diamond)
                .size(10)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("blue").width(1.0)),
        );

    let mut plot = Plot::new();
    plot.add_trace(trace_normal.clone());
    plot.add_trace(trace_timeout.clone());
    plot.add_trace(trace_bar.clone());
    plot.add_trace(trace_proposal_time.clone());
    plot.add_trace(trace_vid_share_time.clone());
    plot.add_trace(trace_dac_time.clone());

    plot.set_layout(
        Layout::new()
            .title("Replica ReplicaStats")
            .auto_size(true)
            .grid(
                LayoutGrid::new()
                    .rows(3)
                    .columns(1)
                    .pattern(GridPattern::Independent),
            )
            .x_axis(Axis::new().title("View"))
            .y_axis(Axis::new().title("Proposal Received Timestamp (s)"))
            .x_axis2(Axis::new().title("Event").domain(&[0.0, 0.5]))
            .y_axis2(Axis::new().title("First Event Count"))
            .x_axis3(Axis::new().title("View"))
            .y_axis3(Axis::new().title("DAC, VID, Proposal Timestamps (s)"))
            .height(2500)
            .margin(layout::Margin::new().left(130)),
    );

    plot.write_html("replica_stats.html");

    println!("Plot saved to replica_stats.html");

    Ok(())
}

fn generate_replica_stats(
    replica_view_stats: &BTreeMap<ViewNumber, ReplicaViewStats<SeqTypes>>,
) -> ReplicaStats {
    let mut vid_vs_dac_order_stats = HashMap::new();
    let mut proposal_vs_vid_order_stats = HashMap::new();
    let mut proposal_vs_dac_order_stats = HashMap::new();

    for record in replica_view_stats.values() {
        if let (Some(vid), Some(dac)) = (record.vid_share_recv, record.da_certificate_recv) {
            let key = if vid < dac {
                "VID received before DAC"
            } else if dac < vid {
                "DAC received before VID"
            } else {
                "VID and DAC received at the same time"
            };
            *vid_vs_dac_order_stats.entry(key.to_string()).or_insert(0) += 1;
        }

        if let (Some(p), Some(v)) = (record.proposal_recv, record.vid_share_recv) {
            let key = if p < v {
                "Proposal received before VID"
            } else if v < p {
                "VID received before Proposal"
            } else {
                "Proposal and VID received at the same time"
            };
            *proposal_vs_vid_order_stats
                .entry(key.to_string())
                .or_insert(0) += 1;
        }

        if let (Some(p), Some(d)) = (record.proposal_recv, record.da_certificate_recv) {
            let key = if p < d {
                "Proposal received before DAC"
            } else if d < p {
                "DAC received before Proposal"
            } else {
                "Proposal and DAC received at the same time"
            };
            *proposal_vs_dac_order_stats
                .entry(key.to_string())
                .or_insert(0) += 1;
        }
    }

    ReplicaStats {
        vid_vs_dac: vid_vs_dac_order_stats,
        proposal_vs_vid: proposal_vs_vid_order_stats,
        proposal_vs_dac: proposal_vs_dac_order_stats,
    }
}

fn print_replica_stats(stats: &ReplicaStats) {
    println!("\n-VID vs DAC:");
    let mut v = stats.vid_vs_dac.iter().collect::<Vec<_>>();
    v.sort_by(|a, b| b.1.cmp(a.1));
    for (k, v) in v {
        println!("{k}: {v} times");
    }

    println!("\n-Proposal vs VID:");
    for (k, v) in &stats.proposal_vs_vid {
        println!("{k}: {v} times");
    }

    println!("\n-Proposal vs DAC:");
    for (k, v) in &stats.proposal_vs_dac {
        println!("{k}: {v} times");
    }
}

fn read_leader_view_stats(
    path: &str,
) -> Result<BTreeMap<ViewNumber, LeaderViewStats<SeqTypes>>, Box<dyn std::error::Error>> {
    println!("\n**--- Leader Stats ---**");
    let mut reader = csv::Reader::from_path(path)?;
    let mut leader_view_stats = BTreeMap::<ViewNumber, LeaderViewStats<SeqTypes>>::new();

    for result in reader.deserialize() {
        let record: LeaderViewStats<SeqTypes> = result?;
        leader_view_stats.insert(record.view, record);
    }
    Ok(leader_view_stats)
}

fn plot_leader_stats(
    leader_view_stats: &BTreeMap<ViewNumber, LeaderViewStats<SeqTypes>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut views = Vec::new();

    let mut da_cert_deltas = Vec::new();
    let mut vid_disperse_deltas = Vec::new();
    let mut qc_formed_deltas = Vec::new();

    // For stats
    let mut da_before_vid = 0;
    let mut vid_before_da = 0;
    let mut da_eq_vid = 0;

    for (&view, record) in leader_view_stats.iter() {
        // Skip if either DA or VID is missing
        let (da, vid) = match (record.da_cert_send, record.vid_disperse_send) {
            (Some(da), Some(vid)) => (da, vid),
            _ => continue,
        };

        let block_built = match record.block_built {
            Some(ts) => ts,
            None => continue,
        };

        views.push(view);

        // Track relative ordering
        if let (Some(da), Some(vid)) = (record.da_cert_send, record.vid_disperse_send) {
            if da < vid {
                da_before_vid += 1;
            } else if vid < da {
                vid_before_da += 1;
            } else {
                da_eq_vid += 1;
            }
        }

        // Deltas for current view
        da_cert_deltas.push((da - block_built) as f64 / 1000.0);
        vid_disperse_deltas.push((vid - block_built) as f64 / 1000.0);

        // Delta for QC formed at view+1

        if let Some(qc_formed) = record.qc_formed {
            qc_formed_deltas.push((qc_formed - block_built) as f64 / 1000.0);
        }
    }

    let mut plot = Plot::new();

    let trace_da_cert_deltas = Scatter::new(views.clone(), da_cert_deltas.clone())
        .mode(Mode::Markers)
        .name("DA Cert Δ (ms)")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Diamond)
                .size(10)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("blue").width(1.0)),
        );

    let trace_vid_disperse_deltas = Scatter::new(views.clone(), vid_disperse_deltas.clone())
        .mode(Mode::Markers)
        .name("VID Disperse Δ (ms)")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Square)
                .size(8)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("orange").width(1.0)),
        );

    let trace_qc_formed_deltas = Scatter::new(views.clone(), qc_formed_deltas.clone())
        .mode(Mode::Markers)
        .name("QC Formed Δ (ms)")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Circle)
                .size(6)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("green").width(1.0)),
        );

    plot.add_trace(trace_da_cert_deltas);
    plot.add_trace(trace_vid_disperse_deltas);
    plot.add_trace(trace_qc_formed_deltas);

    plot.set_layout(
        Layout::new()
            .title("Leader Stats")
            .grid(
                LayoutGrid::new()
                    .rows(1)
                    .columns(1)
                    .pattern(GridPattern::Independent),
            )
            .height(1000)
            .x_axis(Axis::new().title("View"))
            .y_axis(Axis::new().title("Δ from Block Built (ms)"))
            .margin(layout::Margin::new().left(130)),
    );

    println!("\n-DAC vs VID Share:");
    println!("DA cert sent before VID: {da_before_vid} times");
    println!("VID sent before DA cert: {vid_before_da} times");
    println!("DA and VID sent at same time: {da_eq_vid} times");

    print_delta_stats("DA Cert:", &da_cert_deltas);
    print_delta_stats("VID Disperse:", &vid_disperse_deltas);
    print_delta_stats("QC Formed:", &qc_formed_deltas);

    plot.write_html("leader_stats.html");
    println!("Plot saved to leader_stats.html");
    Ok(())
}

fn print_delta_stats(label: &str, values: &[f64]) {
    if values.is_empty() {
        println!("\n--- {label} ---\nNo data available.");
        return;
    }

    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let sum: f64 = values.iter().sum();
    let avg = sum / values.len() as f64;

    println!("\n-{label}");
    println!("Count: {}", values.len());
    println!("Min: {min:.2} ms");
    println!("Max: {max:.2} ms");
    println!("Avg: {avg:.2} ms");
}
