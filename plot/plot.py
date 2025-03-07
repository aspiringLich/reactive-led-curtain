from matplotlib.scale import FuncTransform
import matplotlib.pyplot as plt
import matplotlib.image as mpimg
from typing import Callable
import os
import math
import numpy
import toml

os.chdir(os.path.dirname(__file__))

config = toml.load("../config.toml")

ymin, ymax = config["spectrogram"]["min_frequency"], config["spectrogram"]["max_frequency"]
xmin, xmax = 0, config["fft"]["hop_len"] / 44100 * config["spectrogram"]["time_width"]

def plot(
    image_path: str,
    output_path: str,
    forward: Callable[[float], float],
    inverse: Callable[[float], float],
):
    img = mpimg.imread(image_path)

    fig, ax = plt.subplots()

    # ax.set_yscale("function", functions=(forward, inverse))
    ax.set_ylim(ymin, ymax)
    ax.set_xlim(xmin, xmax)
    ax.set_xlabel("Time (s)")
    ax.set_ylabel("Frequency (Hz)")
    ax.yaxis.set_ticks([0, ymax])

    imshow = ax.imshow(img, cmap='magma', aspect='auto',  extent=(xmin, xmax, ymin, ymax), vmin=0, vmax=50)
    bar = fig.colorbar(imshow)
    bar.set_label("Magnitude (dB)")

    fig.savefig(output_path)

# look at app/src/spectrogram::SpectrogramImageSet::new()

def linear(x):
    return x
plot("spectrogram-linear.png", "plot-linear.png", linear, linear)

def weird_log(x):
    B = 1024
    return 1 - numpy.emath.logn(B - B * x + x, B)
def weird_log_inverse(y):
    B = 1024
    return (B ** (1 - y) - 1) / (B - 1)
plot("spectrogram-log.png", "plot-log.png", weird_log, weird_log_inverse)
