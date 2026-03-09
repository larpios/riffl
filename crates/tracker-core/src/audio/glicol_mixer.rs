use glicol_synth::signal::Noise;
use glicol_synth::{AudioContext, AudioContextBuilder, Message};
use petgraph::graph::NodeIndex;

pub struct GlicolMixer {
    pub context: AudioContext<128>,
    pub num_channels: usize,
    pub sample_rate: u32,
    track_nodes: Vec<NodeIndex>,
    left_buf: Vec<f32>,
    right_buf: Vec<f32>,
    buf_pos: usize,
}

impl GlicolMixer {
    pub fn new(num_channels: usize, sample_rate: u32) -> Self {
        let mut context = AudioContextBuilder::<128>::new()
            .sr(sample_rate as usize)
            .channels(2)
            .build();

        let mut track_nodes = Vec::new();

        use glicol_synth::oscillator::SinOsc;
        for i in 0..num_channels {
            let node = if i % 2 == 0 {
                context.add_mono_node(SinOsc::new())
            } else {
                context.add_mono_node(Noise::new(42))
            };
            let dest = context.destination;
            if i % 2 == 0 {
                context.connect(node, dest);
            } else {
                // Gain node is unavailable; skipping for now
                // Add a Mul node for gain control
                let gain_node = context.add_mono_node(glicol_synth::operator::Mul::new(1.0));
                context.connect(node, gain_node);
                context.connect(gain_node, dest);
                track_nodes.push(gain_node);
            }
            context.send_msg(node, Message::SetToNumber(0, 0.0));
            track_nodes.push(node);
        }

        Self {
            context,
            num_channels,
            sample_rate,
            track_nodes,
            left_buf: Vec::new(),
            right_buf: Vec::new(),
            buf_pos: 0,
        }
    }

    pub fn render(&mut self, output: &mut [f32]) {
        for frame in output.chunks_exact_mut(2) {
            if self.buf_pos >= self.left_buf.len() {
                let block = self.context.next_block();
                self.left_buf = block[0].to_vec();
                self.right_buf = block[1].to_vec();
                self.buf_pos = 0;
            }

            if self.buf_pos < self.left_buf.len() {
                frame[0] = self.left_buf[self.buf_pos];
                frame[1] = self.right_buf[self.buf_pos];
                self.buf_pos += 1;
            } else {
                frame[0] = 0.0;
                frame[1] = 0.0;
            }
        }
    }

    pub fn note_on(&mut self, channel: usize, freq: f32) {
        if let Some(&node) = self.track_nodes.get(channel) {
            self.context.send_msg(node, Message::SetToNumber(0, freq));
        }
    }

    pub fn note_off(&mut self, channel: usize) {
        if let Some(&node) = self.track_nodes.get(channel) {
            self.context.send_msg(node, Message::SetToNumber(0, 0.0));
        }
    }
}
