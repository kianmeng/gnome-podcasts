#![allow(warnings)]

use gstreamer_player as gst;
use gtk;
use gtk::prelude::*;

#[derive(Debug, Clone)]
struct PlayerInfo {
    container: gtk::Box,
    show: gtk::Label,
    episode: gtk::Label,
    cover: gtk::Image,
}

#[derive(Debug, Clone)]
struct PlayerTimes {
    container: gtk::Box,
    progressed: gtk::Label,
    duration: gtk::Label,
    separator: gtk::Label,
    scalebar: gtk::Scale,
}

#[derive(Debug, Clone)]
// FIXME: This is a mock till stuff get sorted out.
enum PlayerState {
    Playing,
    Paused,
    Ready,
}

#[derive(Debug, Clone)]
struct PlayerControls {
    container: gtk::Box,
    play: gtk::Button,
    pause: gtk::Button,
    forward: gtk::Button,
    rewind: gtk::Button,
    state: PlayerState,
}

#[derive(Debug, Clone)]
pub struct PlayerWidget {
    player: gst::Player,
    revealer: gtk::Revealer,
    action_bar: gtk::ActionBar,
    controls: PlayerControls,
    timer: PlayerTimes,
    info: PlayerInfo,
}

impl Default for PlayerWidget {
    fn default() -> Self {
        let builder = gtk::Builder::new_from_resource("/org/gnome/Hammond/gtk/player_toolbar.ui");
        let player = gst::Player::new(None, None);
        let revealer = builder.get_object("revealer").unwrap();
        let action_bar = builder.get_object("action_bar").unwrap();

        let buttons = builder.get_object("buttons").unwrap();
        let play = builder.get_object("play_button").unwrap();
        let pause = builder.get_object("pause_button").unwrap();
        let forward = builder.get_object("ff_button").unwrap();
        let rewind = builder.get_object("rewind_button").unwrap();

        let controls = PlayerControls {
            container: buttons,
            play,
            pause,
            forward,
            rewind,
            state: PlayerState::Ready,
        };

        let timer_container = builder.get_object("timer").unwrap();
        let progressed = builder.get_object("progress_time_label").unwrap();
        let duration = builder.get_object("total_duration").unwrap();
        let separator = builder.get_object("separator").unwrap();
        let scalebar = builder.get_object("seek").unwrap();
        let timer = PlayerTimes {
            container: timer_container,
            progressed,
            duration,
            separator,
            scalebar,
        };

        let labels = builder.get_object("info").unwrap();
        let show = builder.get_object("show_label").unwrap();
        let episode = builder.get_object("episode_label").unwrap();
        let cover = builder.get_object("show_cover").unwrap();
        let info = PlayerInfo {
            container: labels,
            show,
            episode,
            cover,
        };

        PlayerWidget {
            player,
            revealer,
            action_bar,
            controls,
            timer,
            info,
        }
    }
}
