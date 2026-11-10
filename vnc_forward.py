#!/usr/bin/env python3
"""Forward LAN VNC connections to the local QEMU VNC server (127.0.0.1:5900).

QEMU binds its VNC server to loopback only; this forwards
<lan-ip>:5901 -> 127.0.0.1:5900 so other machines on the LAN can connect.

The LAN address is auto-detected (current default-route interface) so a DHCP
change does not break the bind. Override with VNC_LISTEN_HOST / VNC_LISTEN_PORT.
Run standalone:  python3 vnc_forward.py
"""
import os
import socket
import threading

TARGET = ("127.0.0.1", 5900)


def detect_lan_ip():
    """Return the IP of the interface used for outbound traffic (no packets sent)."""
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("1.1.1.1", 80))
        return s.getsockname()[0]
    finally:
        s.close()


LISTEN_HOST = os.environ.get("VNC_LISTEN_HOST") or detect_lan_ip()
LISTEN_PORT = int(os.environ.get("VNC_LISTEN_PORT", "5901"))
LISTEN = (LISTEN_HOST, LISTEN_PORT)


def pipe(src, dst):
    try:
        while True:
            data = src.recv(65536)
            if not data:
                break
            dst.sendall(data)
    except OSError:
        pass
    finally:
        for sock in (src, dst):
            try:
                sock.shutdown(socket.SHUT_RDWR)
            except OSError:
                pass


def handle(client):
    try:
        server = socket.create_connection(TARGET, timeout=10)
    except OSError as exc:
        print("upstream connect failed (is the VM running?):", exc, flush=True)
        client.close()
        return
    threading.Thread(target=pipe, args=(client, server), daemon=True).start()
    pipe(server, client)


def main():
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as listener:
        listener.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
        listener.bind(LISTEN)
        listener.listen(16)
        print(f"forwarding {LISTEN[0]}:{LISTEN[1]} -> {TARGET[0]}:{TARGET[1]}", flush=True)
        while True:
            client, addr = listener.accept()
            print("client connected:", addr, flush=True)
            threading.Thread(target=handle, args=(client,), daemon=True).start()


if __name__ == "__main__":
    main()
