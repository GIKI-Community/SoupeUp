def main():
    import platform
    import socket

    return {
        "message": "hello from Cluster Runtime",
        "hostname": socket.gethostname(),
        "platform": platform.platform(),
    }