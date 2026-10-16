import os
import subprocess
import tempfile
import time
from pathlib import Path
if not Path('/run/.containerenv').exists():
    raise SystemExit('Run this test inside the UI test container.')
root = Path(__file__).resolve().parents[1]
artifacts = Path(os.environ.get('UI_TEST_SHOTS', root / 'target/ui-bar-width'))
artifacts.mkdir(parents=True, exist_ok=True)
runtime = Path(tempfile.mkdtemp(prefix='niri-', dir=artifacts))
app_binary = os.environ.get('STARSHELL_BIN', str(root / 'target/debug/starshell'))
niri_binary = os.environ.get('NIRI_BIN', 'niri')
runtime.chmod(448)
os.environ.update(XDG_RUNTIME_DIR=str(runtime), XDG_CONFIG_HOME=str(runtime / 'config'))
for key in ('WAYLAND_DISPLAY', 'DISPLAY', 'SWAYSOCK', 'NIRI_SOCKET', 'HYPRLAND_INSTANCE_SIGNATURE'):
    os.environ.pop(key, None)
config = runtime / 'sway.conf'
config.write_text('output * resolution 1280x800\noutput * bg #223344 solid_color\n')
swaylog = (runtime / 'sway.log').open('w')
sway = subprocess.Popen(['sway', '--unsupported-gpu', '-c', str(config)], stdout=swaylog, stderr=swaylog, env=dict(os.environ, WLR_BACKENDS='headless', WLR_LIBINPUT_NO_DEVICES='1', WLR_RENDERER='gles2'))
niri = None
app = None
try:
    for _ in range(100):
        sockets = [p for p in runtime.glob('wayland-*') if not p.name.endswith('.lock')]
        if sockets:
            os.environ['WAYLAND_DISPLAY'] = sockets[0].name
            break
        time.sleep(0.1)
    else:
        raise RuntimeError((runtime / 'sway.log').read_text())
    outer = os.environ['WAYLAND_DISPLAY']
    nconfig = runtime / 'niri.kdl'
    scale = os.environ.get('UI_TEST_SCALE', '1.5')
    nconfig.write_text(f'prefer-no-csd\noutput "winit" {{ scale {scale}; }}\nhotkey-overlay {{ skip-at-startup; }}\n')
    nirilog = (runtime / 'niri.log').open('w')
    niri = subprocess.Popen([niri_binary, '-c', str(nconfig)], stdout=nirilog, stderr=nirilog)
    for _ in range(100):
        sockets = [p for p in runtime.glob('wayland-*') if not p.name.endswith('.lock') and p.name != outer]
        if sockets:
            os.environ['WAYLAND_DISPLAY'] = sockets[0].name
            break
        if niri.poll() is not None:
            raise RuntimeError((runtime / 'niri.log').read_text())
        time.sleep(0.1)
    else:
        raise RuntimeError((runtime / 'niri.log').read_text())
    os.environ['NIRI_SOCKET'] = str(next(runtime.glob('niri.*.sock')))
    appconfig = runtime / 'config/starshell/config.toml'
    appconfig.parent.mkdir(parents=True)
    appconfig.write_text('[bar]\nautohide = false\n[ui]\nreduce_motion = true\n')
    applog = (runtime / 'app.log').open('w')
    app = subprocess.Popen([app_binary], stdout=applog, stderr=applog)
    time.sleep(4)

    def assert_full_width(expected_width, name):
        screenshot_env = dict(os.environ, WAYLAND_DISPLAY=outer)
        subprocess.run(['grim', str(runtime / f'{name}.png')], check=True, env=screenshot_env)
        shot = subprocess.run(['grim', '-t', 'ppm', '-'], check=True, capture_output=True, env=screenshot_env).stdout
        header, dimensions, maximum, pixels = shot.split(b'\n', 3)
        width, height = map(int, dimensions.split())
        assert width == expected_width, (width, expected_width)

        def pixel(x, y):
            offset = (y * width + x) * 3
            return pixels[offset:offset + 3]
        surface = pixel(256, 40)
        assert surface != pixel(256, 150), ('bar missing', scale)
        assert pixel(2, 40) == surface, ('bar misses left edge', scale)
        assert pixel(width - 3, 40) == surface, ('bar misses right edge', scale)
        print(f'PASS: full bar width {width} at scale {scale}', flush=True)
    assert_full_width(1280, 'initial')
    sway_socket = str(next(runtime.glob('sway-ipc.*.sock')))
    subprocess.run(['swaymsg', '-s', sway_socket, 'output', 'HEADLESS-1', 'resolution', '1600x900'], check=True, capture_output=True)
    time.sleep(1)
    assert_full_width(1600, 'resized')
    for command in ('show-launcher', 'hide', 'show-dashboard', 'hide', 'show-launcher'):
        subprocess.run([app_binary, command], check=True)
        time.sleep(0.5)
    time.sleep(1)
    subprocess.run(['grim', str(runtime / 'outer.png')], check=True, env=dict(os.environ, WAYLAND_DISPLAY=outer))
    log = (runtime / 'app.log').read_text()
    assert 'Failed to open' not in log, 'Window creation failed'
    assert 'panicked at' not in log, 'Background thread panicked'
    assert 'fish: command not found' not in log, 'Missing fish reload'
    assert 'hyprctl: command not found' not in log, 'Missing Hyprland reload'
    assert 'negative or zero values' not in log, 'Invalid viewport destination'
    assert 'ERROR_SURFACE_LOST_KHR' not in log, 'Vulkan surface lost'
    assert app.poll() is None
    print('PASS: niri bar, launcher, dashboard, reopen')
except Exception:
    if (runtime / 'app.log').exists():
        print((runtime / 'app.log').read_text()[:5000])
    raise
finally:
    for process in (app, niri, sway):
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
    print('Artifacts:', runtime)
