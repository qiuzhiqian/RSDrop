# protocol
# discovery
sender:
{
    device:{
        "name":"QT-UDP-HTY",
        "type":"Deepin",
        "id":
    },
    "port":37850,
    "request":true
}

receiver:
{
    device:{
        "name":"QT-UDP-HTY",
        "type":"Deepin",
        "id":
    },
    "port":37850,
    "request":false
}

# send
## handshake

HANDSHAKE1
sender:
socket->write(crypto.localPublicKey());

receiver:
socket->write(crypto.localPublicKey());

HANDSHAKE2,
sender:
{
    "device":{
        "name"
        "device_type"
        "id":
    }
    "files":[
        {
            "filename":
            "size":
            "verity": {
                "type":
                "data"
            }
        }
    ]
}

receiver:
{
    "response"
}
TRANSFERRING,
sender:
[data] ->
FINISHED