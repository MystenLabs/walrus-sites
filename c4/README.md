# Walrus Sites C4 Model

The C4 Model describes the static structure of the software system.
It is a collection of software architecture diagrams that show how components interact with each other.

Use it as a reference to understand the architecture of the Walrus Sites codebase.

Combine it while reading the [docs](https://docs.wal.app/walrus-sites/intro.html)
to get the best out of it.

## How to access the GUI

The C4 Model is described in `workspace.dsl`, following an "architecture as code" principle.
But the code is not very readable, so we use the Structurizr GUI to visualize it.

To access the GUI, you will have to run a docker container:

```bash
# Pull the structurizr image.
docker pull structurizr/lite

# Make sure you run the command from the root directory of the walrus sites repo.
docker run -it --rm -p 8080:8080 -v ./c4:/usr/local/structurizr structurizr/lite
```

Then, open your browser and go to `http://localhost:8080/`.
