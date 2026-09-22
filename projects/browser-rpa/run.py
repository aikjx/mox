import os

import uvicorn

from rpa import config

if __name__ == "__main__":
    config.ensure_dirs()
    uvicorn.run("rpa.api:app", host=config.HOST,
                port=int(os.environ.get("RPA_PORT", config.PORT)), log_level="info")
