FROM ubuntu

COPY ./ .

RUN apt update -y && apt upgrade -y
RUN apt install -y python3

ENTRYPOINT python3 test.py
