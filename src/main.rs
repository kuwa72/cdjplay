use anyhow::{Context, Result};
use eframe::egui;
use rodio::{Decoder, OutputStream, Sink, OutputStreamBuilder};
use std::fs::File;
use std::io::{Read, Cursor};
use std::path::PathBuf;
use rekordcrate::pdb::{PlaylistTreeNodeId, Track, TrackId};
use rekordcrate::device::{Pdb, PlaylistNode};

struct MediaApp {
    usb_root: Option<PathBuf>,
    pdb: Option<Pdb>,
    playlists: Vec<PlaylistNode>,
    selected_playlist: Option<PlaylistTreeNodeId>,
    tracks_in_playlist: Vec<Track>,
    
    // Audio engine
    _stream: Option<OutputStream>,
    sink: Option<Sink>,
    
    currently_playing: Option<TrackId>,
    is_playing: bool,
    volume: f32,
}

impl MediaApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let stream = match OutputStreamBuilder::open_default_stream() {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("Failed to open audio output: {}", e);
                None
            }
        };

        let sink = stream.as_ref().map(|s| Sink::connect_new(s.mixer()));

        Self {
            usb_root: None,
            pdb: None,
            playlists: Vec::new(),
            selected_playlist: None,
            tracks_in_playlist: Vec::new(),
            _stream: stream,
            sink: sink,
            currently_playing: None,
            is_playing: false,
            volume: 1.0,
        }
    }

    fn load_usb(&mut self, root: PathBuf) -> Result<()> {
        let pdb_path = root.join("PIONEER").join("rekordbox").join("export.pdb");
        let pdb = Pdb::open_from_path(&pdb_path)
            .with_context(|| format!("Failed to open PDB at {}", pdb_path.display()))?;
        
        self.playlists = pdb.get_playlists()?;
        self.pdb = Some(pdb);
        self.usb_root = Some(root);
        Ok(())
    }

    fn select_playlist(&mut self, id: PlaylistTreeNodeId) {
        if let Some(pdb) = &self.pdb {
            self.selected_playlist = Some(id);
            let entries = pdb.get_playlist_entries(id);
            let all_tracks: std::collections::HashMap<TrackId, &Track> = pdb.get_tracks()
                .map(|t| (t.id, t))
                .collect();
            
            // Collect entries into a vector first
            let mut entries_vec: Vec<_> = entries.collect();
            entries_vec.sort_by_key(|(idx, _)| *idx);

            self.tracks_in_playlist = entries_vec
                .into_iter()
                .filter_map(|(_, track_id)| all_tracks.get(&track_id).copied().cloned())
                .collect();
        }
    }

    fn play_track(&mut self, track: &Track) -> Result<()> {
        if let Some(usb_root) = &self.usb_root {
            let rel_path = track.offsets.inner.file_path.clone().into_string()?;
            println!("Attempting to play track: {}", rel_path);
            
            // Paths in PDB are usually relative to the USB root and start with / or \
            let clean_rel_path = rel_path.trim_start_matches('/').trim_start_matches('\\').replace("/", "\\");
            let abs_path = usb_root.join(&clean_rel_path);
            
            println!("Resolved absolute path: {}", abs_path.display());
            
            if !abs_path.exists() {
                anyhow::bail!("Audio file does not exist at: {}", abs_path.display());
            }

            if let Some(sink) = &self.sink {
                let mut file = File::open(&abs_path)
                    .with_context(|| format!("Failed to open audio file: {}", abs_path.display()))?;
                
                // Read entire file into memory to avoid seek issues on USB drives
                let mut data = Vec::new();
                let bytes_read = file.read_to_end(&mut data)
                    .with_context(|| format!("Failed to read audio data from {}", abs_path.display()))?;
                
                println!("Read {} bytes into memory", bytes_read);
                
                if bytes_read == 0 {
                    anyhow::bail!("Audio file is empty: {}", abs_path.display());
                }

                let cursor = Cursor::new(data);
                let source = Decoder::new(cursor)
                    .with_context(|| format!("Failed to create decoder for {}", abs_path.display()))?;
                
                sink.stop();
                sink.append(source);
                sink.play();
                self.currently_playing = Some(track.id);
                self.is_playing = true;
            }
        }
        Ok(())
    }
}

impl eframe::App for MediaApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            if ui.button("Open USB Root").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    if let Err(e) = self.load_usb(path) {
                        eprintln!("{}", e);
                    }
                }
            }

            ui.separator();

            ui.label("Playlists");
            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut playlist_to_select = None;
                for node in &self.playlists {
                    display_playlist_node(ui, node, &mut playlist_to_select);
                }
                if let Some(id) = playlist_to_select {
                    self.select_playlist(id);
                }
            });
        });

        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(sink) = &self.sink {
                    if ui.button(if self.is_playing { "Pause" } else { "Play" }).clicked() {
                        if self.is_playing {
                            sink.pause();
                        } else {
                            sink.play();
                        }
                        self.is_playing = !self.is_playing;
                    }
                    
                    if ui.add(egui::Slider::new(&mut self.volume, 0.0..=1.0).text("Volume")).changed() {
                        sink.set_volume(self.volume);
                    }
                }

                if let Some(track_id) = self.currently_playing {
                    ui.label(format!("Playing: Track {:?}", track_id.0));
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Tracks");
            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("track_grid").striped(true).show(ui, |ui| {
                    ui.label("ID");
                    ui.label("Title");
                    ui.end_row();

                    let mut track_to_play = None;
                    for track in &self.tracks_in_playlist {
                        if ui.button(format!("{}", track.id.0)).clicked() {
                            track_to_play = Some(track.clone());
                        }
                        if let Ok(title) = track.offsets.inner.title.clone().into_string() {
                            ui.label(title);
                        } else {
                            ui.label("<Invalid String>");
                        }
                        ui.end_row();
                    }

                    if let Some(track) = track_to_play {
                        if let Err(e) = self.play_track(&track) {
                            eprintln!("Playback error: {:?}", e);
                        }
                    }
                });
            });
        });
    }
}

fn display_playlist_node(ui: &mut egui::Ui, node: &PlaylistNode, playlist_to_select: &mut Option<PlaylistTreeNodeId>) {
    match node {
        PlaylistNode::Folder(folder) => {
            egui::CollapsingHeader::new(&folder.name).show(ui, |ui| {
                for child in &folder.children {
                    display_playlist_node(ui, child, playlist_to_select);
                }
            });
        }
        PlaylistNode::Playlist(playlist) => {
            if ui.selectable_label(false, &playlist.name).clicked() {
                *playlist_to_select = Some(playlist.id);
            }
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1024.0, 768.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Rekordbox Media Player",
        options,
        Box::new(|cc| Ok(Box::new(MediaApp::new(cc)))),
    )
}
