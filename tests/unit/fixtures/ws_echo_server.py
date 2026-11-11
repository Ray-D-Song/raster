#!/usr/bin/env python3
"""Minimal RFC 6455 echo server for exercising the WebSocket client.

Standard library only: a handshake, then text/binary echo with fragmentation,
ping/pong and close handling. Not a general-purpose server; it exists so the
client can be tested against a real peer.
"""
import base64
import hashlib
import socket
import struct
import sys
import threading

GUID = b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11"
MODE = "echo"


def handshake(conn):
    data = b""
    while b"\r\n\r\n" not in data:
        chunk = conn.recv(4096)
        if not chunk:
            return False
        data += chunk
    head, _, rest = data.partition(b"\r\n\r\n")
    headers = {}
    for line in head.split(b"\r\n")[1:]:
        if b":" in line:
            key, _, value = line.partition(b":")
            headers[key.strip().lower()] = value.strip()
    if headers.get(b"upgrade", b"").lower() != b"websocket":
        conn.sendall(b"HTTP/1.1 400 Bad Request\r\n\r\n")
        return False
    key = headers.get(b"sec-websocket-key")
    if not key:
        conn.sendall(b"HTTP/1.1 400 Bad Request\r\n\r\n")
        return False
    accept = base64.b64encode(hashlib.sha1(key + GUID).digest())
    # A deliberately wrong accept token, for the negative test.
    if headers.get(b"x-test-bad-accept") == b"1":
        accept = base64.b64encode(b"\x00" * 20)
    response = (
        b"HTTP/1.1 101 Switching Protocols\r\n"
        b"Upgrade: websocket\r\n"
        b"Connection: Upgrade\r\n"
        b"Sec-WebSocket-Accept: " + accept + b"\r\n\r\n"
    )
    conn.sendall(response)
    return rest, headers


def recv_exact(conn, count):
    buf = b""
    while len(buf) < count:
        chunk = conn.recv(count - len(buf))
        if not chunk:
            raise ConnectionError("peer closed")
        buf += chunk
    return buf


def read_frame(conn):
    b1, b2 = recv_exact(conn, 2)
    fin = b1 & 0x80
    opcode = b1 & 0x0F
    masked = b2 & 0x80
    length = b2 & 0x7F
    if length == 126:
        length = struct.unpack("!H", recv_exact(conn, 2))[0]
    elif length == 127:
        length = struct.unpack("!Q", recv_exact(conn, 8))[0]
    mask = recv_exact(conn, 4) if masked else None
    payload = recv_exact(conn, length) if length else b""
    if mask:
        payload = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
    return fin, opcode, payload


def send_frame(conn, opcode, payload, fin=True):
    header = bytes([(0x80 if fin else 0) | opcode])
    length = len(payload)
    if length < 126:
        header += bytes([length])
    elif length < 65536:
        header += bytes([126]) + struct.pack("!H", length)
    else:
        header += bytes([127]) + struct.pack("!Q", length)
    conn.sendall(header + payload)


def serve(conn):
    try:
        rest, headers = handshake(conn)
        if rest is False:
            return
        pending = rest

        # Test hook: FRAGMENT_MODE makes every connection receive a fragmented
        # message plus a ping before echoing anything.
        if MODE == "fragmented":
            def frame(op, payload, fin=True):
                header = bytes([(0x80 if fin else 0) | op])
                length = len(payload)
                if length < 126:
                    header += bytes([length])
                elif length < 65536:
                    header += bytes([126]) + struct.pack("!H", length)
                else:
                    header += bytes([127]) + struct.pack("!Q", length)
                return header + payload
            conn.sendall(
                frame(0x1, b"he", fin=False)
                + frame(0x0, b"ll", fin=False)
                + frame(0x0, b"o", fin=True)
                + frame(0x9, b"pingdata")
            )
            # Expect the pong back, then close.
            try:
                fin, opcode, payload = read_frame(conn)
                sys.stderr.write("pong-opcode:%d\n" % opcode)
                sys.stderr.flush()
            except Exception:
                pass
            conn.sendall(frame(0x8, struct.pack("!H", 1000)))
            return
        conn.settimeout(30)

        def buffered_read():
            nonlocal pending
            # Feed bytes already consumed by the handshake first.
            if pending:
                b1, b2 = pending[0], pending[1]
                idx = 2
                fin = b1 & 0x80
                opcode = b1 & 0x0F
                masked = b2 & 0x80
                length = b2 & 0x7F
                if length == 126:
                    length = struct.unpack("!H", pending[idx:idx + 2])[0]
                    idx += 2
                elif length == 127:
                    length = struct.unpack("!Q", pending[idx:idx + 8])[0]
                    idx += 8
                mask = pending[idx:idx + 4] if masked else None
                idx += 4 if masked else 0
                payload = pending[idx:idx + length]
                pending = pending[idx + length:]
                if mask:
                    payload = bytes(b ^ mask[i % 4] for i, b in enumerate(payload))
                return fin, opcode, payload
            return read_frame(conn)

        fragments = []
        frag_opcode = None
        while True:
            fin, opcode, payload = buffered_read()

            if opcode == 0x8:  # close
                code = payload[:2] if len(payload) >= 2 else b""
                send_frame(conn, 0x8, code)
                return
            if opcode == 0x9:  # ping
                send_frame(conn, 0xA, payload)
                continue
            if opcode == 0xA:  # pong
                continue

            if opcode in (0x1, 0x2):
                fragments = [payload]
                frag_opcode = opcode
            elif opcode == 0x0:
                fragments.append(payload)

            if not fin:
                continue

            message = b"".join(fragments)
            fragments = []
            send_frame(conn, frag_opcode, message)
    except (ConnectionError, socket.timeout, OSError):
        pass
    finally:
        conn.close()


def main():
    global MODE
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 0
    if len(sys.argv) > 2:
        MODE = sys.argv[2]
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    server.bind(("127.0.0.1", port))
    server.listen(8)
    print(server.getsockname()[1], flush=True)
    while True:
        conn, _ = server.accept()
        threading.Thread(target=serve, args=(conn,), daemon=True).start()


if __name__ == "__main__":
    main()
