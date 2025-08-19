use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use espresso_types::SeqTypes;
use hotshot_task_impls::stats::{LeaderViewStats, ReplicaViewStats};
use hotshot_types::data::ViewNumber;
use plotly::{
    common::{HoverInfo, Line, Marker, MarkerSymbol, Mode},
    layout::{self, Axis, GridPattern, LayoutGrid},
    Bar, Layout, Plot, Scatter,
};

#[derive(Parser)]
#[command(author, version, about)]
struct Command {
    #[command(subcommand)]
    subcommand: SubCommands,
}

#[derive(Subcommand)]
enum SubCommands {
    /// Analyze replica stats from a CSV file
    Replica {
        /// Path to the replica stats CSV file
        path: PathBuf,
        /// Output HTML file (default: replica_stats.html)
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Analyze leader stats from a CSV file
    Leader {
        /// Path to the leader stats CSV file
        path: PathBuf,
        /// Output HTML file (default: leader_stats.html)
        #[arg(long)]
        output: Option<PathBuf>,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let command = Command::parse();

    match command.subcommand {
        SubCommands::Replica { path, output } => {
            let replica_view_stats = read_replica_view_stats(&path)?;
            plot_replica_stats(
                &replica_view_stats,
                output
                    .as_deref()
                    .unwrap_or_else(|| Path::new("replica_stats.html")),
            )?;
            let stats = generate_replica_stats(&replica_view_stats);
            print_replica_stats(&stats);
        },
        SubCommands::Leader { path, output } => {
            let leader_view_stats = read_leader_view_stats(&path)?;
            plot_and_print_leader_stats(
                &leader_view_stats,
                output
                    .as_deref()
                    .unwrap_or_else(|| Path::new("leader_stats.html")),
            )?;
        },
    }

    Ok(())
}

struct ReplicaStats {
    /// Time between view change and VID share received (ms)
    pub vid_deltas_from_vc: Vec<f64>,
    /// Time between view change and DA certificate received (ms)
    pub dac_deltas_from_vc: Vec<f64>,
    /// Time between view change and proposal received (ms)
    pub proposal_deltas_from_vc: Vec<f64>,
}

/// Read replica stats from CSV into a BTreeMap
fn read_replica_view_stats(
    path: &Path,
) -> Result<BTreeMap<ViewNumber, ReplicaViewStats<SeqTypes>>, Box<dyn std::error::Error>> {
    println!("\n**--- Replica Stats ---**");
    let mut reader = csv::Reader::from_path(path)
        .map_err(|e| format!("Failed to open replica stats CSV at {path:?}: {e}"))?;
    let mut replica_view_stats = BTreeMap::new();

    for result in reader.deserialize() {
        let record: ReplicaViewStats<SeqTypes> = result?;
        replica_view_stats.insert(record.view, record);
    }

    Ok(replica_view_stats)
}

/// Generate plots of replica stats
fn plot_replica_stats(
    replica_view_stats: &BTreeMap<ViewNumber, ReplicaViewStats<SeqTypes>>,
    output_file: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut x_views = Vec::new();
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
        // Skip views without a proposal receive event
        let proposal_ts = match record.proposal_recv {
            Some(t) => t,
            None => continue,
        };

        // Collect all events with timestamps in milliseconds
        let mut events_with_ts = Vec::new();
        if let Some(ts) = record.proposal_recv {
            events_with_ts.push(("proposal_recv", ts / 1_000_000));
        }
        if let Some(ts) = record.vote_send {
            events_with_ts.push(("vote_send", ts / 1_000_000));
        }
        if let Some(ts) = record.timeout_vote_send {
            events_with_ts.push(("timeout_vote_send", ts / 1_000_000));
        }
        if let Some(ts) = record.da_proposal_received {
            events_with_ts.push(("da_proposal_received", ts / 1_000_000));
        }
        if let Some(ts) = record.da_proposal_validated {
            events_with_ts.push(("da_proposal_validated", ts / 1_000_000));
        }
        if let Some(ts) = record.da_certificate_recv {
            events_with_ts.push(("da_certificate_recv", ts / 1_000_000));
        }
        if let Some(ts) = record.proposal_prelim_validated {
            events_with_ts.push(("proposal_prelim_validated", ts / 1_000_000));
        }
        if let Some(ts) = record.proposal_validated {
            events_with_ts.push(("proposal_validated", ts / 1_000_000));
        }
        if let Some(ts) = record.timeout_triggered {
            events_with_ts.push(("timeout_triggered", ts / 1_000_000));
        }
        if let Some(ts) = record.vid_share_validated {
            events_with_ts.push(("vid_share_validated", ts / 1_000_000));
        }
        if let Some(ts) = record.vid_share_recv {
            events_with_ts.push(("vid_share_recv", ts / 1_000_000));
        }

        // Count which event appeared first
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

        // Separate views where timeout was triggered vs normal
        if record.timeout_triggered.is_some() {
            x_views_timeout.push(view);
            y_timestamps_timeout.push((proposal_ts as f64) / 1_000_000_000.0);
            hover_texts_timeout.push(hover);
        } else {
            x_views.push(view);
            y_timestamps_normal.push((proposal_ts as f64) / 1_000_000_000.0);
            hover_texts_normal.push(hover);
        }

        views.push(view);
        proposal_times.push(record.proposal_recv.map(|t| t as f64));
        vid_share_times.push(record.vid_share_recv.map(|t| t as f64));
        dac_times.push(record.da_certificate_recv.map(|t| t as f64));
    }

    // Aggregate first event stats for bar chart
    let mut first_events: Vec<_> = first_event_counts.into_iter().collect();
    first_events.sort_by(|a, b| b.1.cmp(&a.1));
    let (bar_labels, bar_values): (Vec<_>, Vec<_>) = first_events
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .unzip();

    let trace_normal = Scatter::new(x_views, y_timestamps_normal)
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

    // scatter plots for Proposal / VID / DAC against on same graph with different traces
    // against the view on x-axis
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

    plot.write_html(output_file);
    println!("Plot saved to {output_file:?}");

    Ok(())
}

/// Computes replica stats
/// it generates the time difference for VID/DAC/Proposal
/// from the view change event
fn generate_replica_stats(
    replica_view_stats: &BTreeMap<ViewNumber, ReplicaViewStats<SeqTypes>>,
) -> ReplicaStats {
    let mut vid_deltas_from_vc = Vec::new();
    let mut dac_deltas_from_vc = Vec::new();
    let mut proposal_deltas_from_vc = Vec::new();

    for record in replica_view_stats.values() {
        if let Some(vc) = record.view_change {
            if let Some(vid) = record.vid_share_recv {
                let delta_ms = (vid - vc) as f64 / 1_000_000.0;
                vid_deltas_from_vc.push(delta_ms);
            }

            if let Some(dac) = record.da_certificate_recv {
                let delta_ms = (dac - vc) as f64 / 1_000_000.0;
                dac_deltas_from_vc.push(delta_ms);
            }

            if let Some(prop) = record.proposal_recv {
                let delta_ms = (prop - vc) as f64 / 1_000_000.0;
                proposal_deltas_from_vc.push(delta_ms);
            }
        }
    }

    ReplicaStats {
        vid_deltas_from_vc,
        dac_deltas_from_vc,
        proposal_deltas_from_vc,
    }
}

fn print_replica_stats(stats: &ReplicaStats) {
    println!("Deltas calculated from view change time:");
    print_delta_stats("DA Cert:", &stats.dac_deltas_from_vc);
    print_delta_stats("VID Disperse:", &stats.vid_deltas_from_vc);
    print_delta_stats("Proposal Received:", &stats.proposal_deltas_from_vc);
}

/// Read leader stats from CSV into a BTreeMap
fn read_leader_view_stats(
    path: &Path,
) -> Result<BTreeMap<ViewNumber, LeaderViewStats<SeqTypes>>, Box<dyn std::error::Error>> {
    println!("\n**--- Leader Stats ---**");
    let mut reader = csv::Reader::from_path(path)
        .map_err(|e| format!("Failed to open leader stats CSV at {path:?}: {e}"))?;
    let mut leader_view_stats = BTreeMap::<ViewNumber, LeaderViewStats<SeqTypes>>::new();

    for result in reader.deserialize() {
        let record: LeaderViewStats<SeqTypes> = result?;
        leader_view_stats.insert(record.view, record);
    }
    Ok(leader_view_stats)
}

fn plot_and_print_leader_stats(
    leader_view_stats: &BTreeMap<ViewNumber, LeaderViewStats<SeqTypes>>,
    output_file: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut views = Vec::new();
    let mut vid_ts = Vec::new();
    let mut dac_ts = Vec::new();
    let mut qc_ts = Vec::new();

    // Deltas relative to block built
    let mut da_cert_deltas = Vec::new();
    let mut vid_disperse_deltas = Vec::new();
    // Deltas relative to previous proposal received
    let mut block_built_prev_prop_deltas = Vec::new();

    // For stats
    let mut first_event_counts = HashMap::new();
    let mut qc_vid_diffs = Vec::new();
    let mut qc_dac_diffs = Vec::new();
    let mut vid_dac_diffs = Vec::new();

    for (&view, record) in leader_view_stats.iter() {
        // Skip if either Block built, DA, VID, or QC is missing
        let (block_built, dac, vid, qc) = match (
            record.block_built,
            record.da_cert_send,
            record.vid_disperse_send,
            record.qc_formed,
        ) {
            (Some(block_built), Some(dac), Some(vid), Some(qc)) => (block_built, dac, vid, qc),
            _ => continue, // need all three
        };

        views.push(view);

        // Determine first among QC / VID / DAC
        let mut events = vec![("QC", qc), ("VID", vid), ("DAC", dac)];
        events.sort_by_key(|&(_, ts)| ts);
        let first = events[0].0;
        *first_event_counts.entry(first).or_insert(0) += 1;

        qc_vid_diffs.push(((qc as i64 - vid as i64) as f64) / 1_000_000.0);
        qc_dac_diffs.push(((qc as i64 - dac as i64) as f64) / 1_000_000.0);
        vid_dac_diffs.push(((vid as i64 - dac as i64) as f64) / 1_000_000.0);

        // Difference relative to block built
        da_cert_deltas.push((dac - block_built) as f64 / 1_000_000.0);
        vid_disperse_deltas.push((vid - block_built) as f64 / 1_000_000.0);

        // Difference between block built and previous proposal
        if let Some(prev_prop) = record.prev_proposal_send {
            block_built_prev_prop_deltas.push((block_built - prev_prop) as f64 / 1_000_000.0);
        }

        vid_ts.push(Some(vid as f64 / 1_000_000_000.0));
        dac_ts.push(Some(dac as f64 / 1_000_000_000.0));
        qc_ts.push(Some(qc as f64 / 1_000_000_000.0));
    }

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

    let mut plot = Plot::new();

    plot.add_trace(trace_da_cert_deltas);
    plot.add_trace(trace_vid_disperse_deltas);

    let trace_block_built_prev_prop =
        Scatter::new(views.clone(), block_built_prev_prop_deltas.clone())
            .mode(Mode::Markers)
            .name("Block Built Δ from previous proposal (ms)")
            .x_axis("x2")
            .y_axis("y2")
            .marker(Marker::new().symbol(MarkerSymbol::Circle));
    plot.add_trace(trace_block_built_prev_prop);

    let trace_vid = Scatter::new(views.clone(), vid_ts)
        .mode(Mode::Markers)
        .name("VID Timestamp")
        .x_axis("x3")
        .y_axis("y3")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Square)
                .size(8)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("orange").width(1.0)),
        );

    let trace_dac = Scatter::new(views.clone(), dac_ts)
        .mode(Mode::Markers)
        .name("DAC Timestamp")
        .x_axis("x3")
        .y_axis("y3")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Diamond)
                .size(10)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("blue").width(1.0)),
        );

    let trace_qc = Scatter::new(views.clone(), qc_ts)
        .mode(Mode::Markers)
        .name("QC Timestamp")
        .x_axis("x3")
        .y_axis("y3")
        .marker(
            Marker::new()
                .symbol(MarkerSymbol::Circle)
                .size(6)
                .color("rgba(0,0,0,0)")
                .line(Line::new().color("green").width(1.0)),
        );

    plot.add_trace(trace_vid);
    plot.add_trace(trace_dac);
    plot.add_trace(trace_qc);

    plot.set_layout(
        Layout::new()
            .title("Leader Stats")
            .grid(
                LayoutGrid::new()
                    .rows(3)
                    .columns(1)
                    .pattern(GridPattern::Independent),
            )
            .height(2000)
            .x_axis(Axis::new().title("View"))
            .y_axis(Axis::new().title("VID/DAC Δ from Block Built (ms)"))
            .x_axis2(Axis::new().title("View"))
            .y_axis2(Axis::new().title("Block built Δ from previous proposal (ms)"))
            .x_axis3(Axis::new().title("View"))
            .y_axis3(Axis::new().title("VID/DAC/QC Timestamps (ms)"))
            .margin(layout::Margin::new().left(130)),
    );

    println!("\nOrdering of VID, DAC, QC):");
    for (event, count) in &first_event_counts {
        println!(" {event} was first in {count} views");
    }

    println!("\nDeltas calculated from block built:");
    print_delta_stats("DA Cert:", &da_cert_deltas);
    print_delta_stats("VID Disperse:", &vid_disperse_deltas);

    println!("\nDeltas calculated from previous proposal:");
    print_delta_stats("Block built:", &block_built_prev_prop_deltas);

    plot.write_html(output_file);
    println!("\n\nPlot saved to {output_file:?}");
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

    println!("\n{label}");
    println!("Count: {}", values.len());
    println!("Min: {min:.2} ms");
    println!("Max: {max:.2} ms");
    println!("Avg: {avg:.2} ms");
}
