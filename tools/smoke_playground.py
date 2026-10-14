#!/usr/bin/env python3
"""Exercise the real local Arena server and both generation profiles."""
import argparse
import json
from pathlib import Path
import socket
import subprocess
import tempfile
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', default='target/release/infernal')
    args = parser.parse_args()
    binary = str(Path(args.binary).resolve())
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0))
        port = sock.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    with tempfile.TemporaryDirectory(prefix='infernal-playground-') as directory:
        process = subprocess.Popen([binary, 'serve', str(port)], cwd=directory)
        try:
            for _ in range(100):
                try:
                    with urlopen(base, timeout=1) as response:
                        html = response.read().decode()
                    break
                except URLError:
                    if process.poll() is not None:
                        raise RuntimeError('Arena server exited before startup')
                    time.sleep(0.1)
            else:
                raise RuntimeError('Arena server did not start')
            assert 'id="profile"' in html and 'value="v2"' in html

            def generate(**extra):
                data = dict(prompt='friendly frog that sprays acid', seed=42, **extra)
                request = Request(base + '/api/generate', json.dumps(data).encode(),
                                  {'Content-Type': 'application/json'})
                with urlopen(request, timeout=180) as response:
                    return json.load(response)

            default = generate()
            v1 = generate(profile='v1')
            v2 = generate(profile='v2')
            replay = generate(profile='v2')
            cbor = generate(profile='v2', format='cbor')
            assert default == v1, 'Default must remain V1'
            assert v2 == replay, 'V2 cached result must be stable'
            assert len({v1['sprites']['url'], v2['sprites']['url'], cbor['sprites']['url']}) == 3
            for monster in (v1, v2, cbor):
                assert monster['animations'] and monster['sprites']['angles']
                with urlopen(base + monster['sprites']['url']) as response:
                    assert response.read().startswith(b'\x89PNG\r\n\x1a\n')
            for invalid in ('v3', 2, None):
                try:
                    generate(profile=invalid)
                except HTTPError as error:
                    assert error.code == 400
                    assert 'profile must be v1 or v2' in error.read().decode()
                else:
                    raise AssertionError('Invalid profile was accepted')
            print('PASS: real V1/V2/CBOR generation, V1 default, separate caches, PNG assets, invalid-profile errors')
        finally:
            process.terminate()
            process.wait(timeout=10)


if __name__ == '__main__':
    main()
