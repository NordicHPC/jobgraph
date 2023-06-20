#![allow(clippy::too_many_arguments)]

#[macro_use]
extern crate anyhow;

use anyhow::Result;
use clap::Parser;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};

use std::io;

use std::collections::HashMap;

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Span, Spans},
    widgets::{Axis, Block, Borders, Chart, Dataset, Paragraph, Wrap},
    Frame, Terminal,
};

mod dates;
mod nodelist;
mod read_data;
mod slurm;

// Time is expressed in seconds since the epoch UTC.
// CPU load is expressed in terms of load on one core by a process, 1.0 == 100% of one core.
// Memory is expressed in terms of usage by a process.
// CPU load and memory usage are summed across all processes for the job, on a single host.
// The "data" map maps a hostname (node name) to the usage data for the job on that node.

pub struct Usage {
    pub time: f64,
    pub cpu_load: f64,
    pub mem_gb: f64,
}

pub struct UsageData {
    pub min_time_h: f64,
    pub max_time_h: f64,
    pub max_cpu_load: f64,
    pub max_memory_gb: f64,
    pub data: HashMap<String, Vec<Usage>>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    // FIXME: later we need a nicer way to configure this on other clusters
    // without recompiling the code
    #[arg(short, long, default_value = "/cluster/shared/sonar/data")]
    data_path: String,

    #[arg(short, long)]
    job_id: String,

    #[arg(long, default_value = "false")]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if !std::path::Path::new(&args.data_path).exists() {
        return Err(anyhow!("The path {} does not exist.", args.data_path));
    }

    let mut job_id = args.job_id;

    let (out, err) = slurm::sacct(&job_id, args.debug);

    if !slurm::job_id_is_valid(&out, &err) {
        return Err(anyhow!(
            "A job with the ID {} does not seem to exist.",
            job_id
        ));
    }

    if slurm::job_id_is_array(&job_id, &out) {
        return Err(anyhow!("Sorry, currently this can't visualize array jobs."));
    }

    if job_id.contains('_') {
        job_id = slurm::array_subjob_id(&out);
    }

    let hostnames = slurm::get_hostnames(&out);
    let dates = slurm::get_dates(&out);

    let requested_memory = slurm::requested_memory(&out);
    let requested_num_cores = slurm::requested_num_cores(&out);

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(
        &mut terminal,
        &args.data_path,
        &job_id,
        &hostnames,
        &dates,
        requested_memory,
        requested_num_cores,
    );

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    data_path: &str,
    job_id: &str,
    hostnames: &[String],
    dates: &[String],
    requested_memory: usize,
    requested_num_cores: usize,
) -> Result<()> {
    let usage_data = read_data::collect_data(data_path, job_id, dates, hostnames)?;

    let num_charts_per_page = (usage_data.data.len()).min(3);
    let num_tabs = (hostnames.len() + num_charts_per_page - 1) / num_charts_per_page;
    let mut tab: isize = 0;

    loop {
        terminal.draw(|f| {
            ui(
                f,
                num_charts_per_page,
                tab as usize,
                num_tabs,
                job_id,
                &usage_data,
                requested_memory,
                requested_num_cores,
            )
        })?;

        if let Event::Key(key) = event::read()? {
            if let KeyCode::Char('q') = key.code {
                return Ok(());
            }
            if let KeyCode::Esc = key.code {
                return Ok(());
            }
            if let KeyCode::Left = key.code {
                tab -= 1;
                if tab < 0 {
                    tab = (num_tabs - 1) as isize;
                }
            }
            if let KeyCode::Right = key.code {
                tab += 1;
                if tab as usize >= num_tabs {
                    tab = 0;
                }
            }
        }
    }
}

fn ui<B: Backend>(
    f: &mut Frame<B>,
    num_charts_per_page: usize,
    tab: usize,
    num_tabs: usize,
    job_id: &str,
    usage_data: &UsageData,
    requested_memory: usize,
    requested_num_cores: usize,
) {
    let data = &usage_data.data;
    let min_time_h = usage_data.min_time_h;
    let max_time_h = usage_data.max_time_h;
    let max_cpu_load = usage_data.max_cpu_load;
    let max_memory_gb = usage_data.max_memory_gb;
    let main_chunks = Layout::default()
        .constraints([Constraint::Length(5), Constraint::Min(0)].as_ref())
        .split(f.size());

    draw_info_box(
        f,
        tab,
        num_tabs,
        main_chunks[0],
        job_id,
        requested_memory,
        requested_num_cores,
    );

    let constraints_vec = data
        .iter()
        .map(|_| Constraint::Percentage(100 / num_charts_per_page as u16))
        .collect::<Vec<_>>();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints_vec.as_ref())
        .split(main_chunks[1]);

    let mut hostnames = data.keys().collect::<Vec<_>>();

    // make sure that hostnames show up sorted and not randomly arranged
    hostnames.sort();

    for (i, &hostname) in hostnames.iter().enumerate() {
        if i / num_charts_per_page != tab {
            continue;
        }

        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
            .split(chunks[i % num_charts_per_page]);

        let usage = data.get(hostname).unwrap();

        let cpu_data = usage
            .iter()
            .map(|Usage { time, cpu_load, .. }| (*time, *cpu_load))
            .collect::<Vec<_>>();

        draw_chart(
            f,
            columns[0],
            hostname,
            "time",
            "CPU load",
            &cpu_data,
            min_time_h,
            max_time_h,
            max_cpu_load,
        );

        let memory_data = usage
            .iter()
            .map(|Usage { time, mem_gb, .. }| (*time, *mem_gb))
            .collect::<Vec<_>>();

        draw_chart(
            f,
            columns[1],
            hostname,
            "time",
            "Memory (GiB)",
            &memory_data,
            min_time_h,
            max_time_h,
            max_memory_gb,
        );
    }
}

fn draw_chart<B: Backend>(
    f: &mut Frame<B>,
    area: Rect,
    hostname: &str,
    x_title: &str,
    y_title: &str,
    data: &[(f64, f64)],
    // allocated_value: f64,
    x_min: f64,
    x_max: f64,
    y_max: f64,
) {
    // let num_steps = 500;
    // let step = (x_max - x_min) / num_steps as f64;
    // let allocated = (0..=num_steps)
    //     .map(|i| (x_min + step * (i as f64), allocated_value))
    //     .collect::<Vec<_>>();

    let datasets = vec![
        // Dataset::default()
        //     .name("allocated")
        //     .marker(symbols::Marker::Dot)
        //     .style(Style::default().fg(Color::Cyan))
        //     .data(&allocated),
        Dataset::default()
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .data(data),
    ];

    let time_difference = x_max - x_min;

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .title(Span::styled(
                    hostname,
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ))
                .borders(Borders::ALL),
        )
        .x_axis(
            Axis::default()
                .title(x_title)
                .style(Style::default().fg(Color::Gray))
                .bounds([x_min, x_max])
                .labels(vec![
                    Span::raw("start"),
                    Span::raw(format!("{:.1} hours", time_difference / 2.0)),
                    Span::raw(format!("{:.1} hours", time_difference)),
                ]),
        )
        .y_axis(
            Axis::default()
                .title(y_title)
                .style(Style::default().fg(Color::Gray))
                .bounds([0.0, 1.2 * y_max])
                .labels(vec![
                    Span::raw(""),
                    Span::raw(format!("{:.1}", 1.2 * y_max / 2.0)),
                    Span::raw(format!("{:.1}", 1.2 * y_max)),
                ]),
        );

    f.render_widget(chart, area);
}

fn draw_info_box<B: Backend>(
    f: &mut Frame<B>,
    tab: usize,
    num_tabs: usize,
    area: Rect,
    job_id: &str,
    requested_memory: usize,
    requested_num_cores: usize,
) {
    let mut text = Vec::new();

    text.push(Spans::from(vec![
        Span::raw("Requested number of cores: "),
        Span::styled(
            format!("{}", requested_num_cores),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
    ]));

    text.push(Spans::from(vec![
        Span::raw("Requested memory/core: "),
        Span::styled(
            format!("{}", requested_memory),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ),
        Span::raw(" MB"),
    ]));

    let mut line = Vec::new();

    if num_tabs > 1 {
        line.push(Span::styled(
            format!("Tab: {}/{}", tab + 1, num_tabs),
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(Color::Magenta),
        ));
        line.push(Span::raw(" "));
        line.push(Span::styled(
            "(navigate with left/right arrow)",
            Style::default().fg(Color::Magenta),
        ));
        line.push(Span::raw(" "));
    }

    line.push(Span::styled(
        "Quit with 'q' or Esc",
        Style::default()
            .add_modifier(Modifier::BOLD)
            .fg(Color::Green),
    ));

    text.push(Spans::from(line));

    let block = Block::default().borders(Borders::ALL).title(Span::styled(
        format!("Job {}", job_id),
        Style::default()
            .fg(Color::Magenta)
            .add_modifier(Modifier::BOLD),
    ));
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}
