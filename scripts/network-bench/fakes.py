"""Test doubles shared by the network-bench tests. Never imported by `netbench.py` or `aws-bench`,
which ship to hosts without this file."""

import json
from typing import Any

SINGLE_MANIFEST_MEDIA_TYPE = "application/vnd.oci.image.manifest.v1+json"
INDEX_MEDIA_TYPE = "application/vnd.oci.image.index.v1+json"
JSON_MEDIA_TYPE = "application/json"

Reply = tuple[int, dict[str, str], bytes]


def _json_reply(obj: dict[str, Any], content_type: str = JSON_MEDIA_TYPE) -> Reply:
    return 200, {"Content-Type": content_type}, json.dumps(obj).encode()


class FakeRegistry:
    """A minimal OCI/Docker registry usable as the `fetch` argument of `resolve_image`: one
    repository and tag, anonymous token challenge. `platforms` is a list of `(os,
    architecture)` pairs for the index; a `tag` of `"missing"` makes the manifest request 404,
    `deny_token` makes the token endpoint 401 (a private image), `deny_manifest_status` makes
    the manifest request fail with that status without ever offering a token challenge, and
    `index=False` serves a single manifest at the tag instead of a multi-platform index."""

    def __init__(
        self,
        repository: str,
        tag: str,
        platforms: list[tuple[str, str]],
        revision: str | None = None,
        deny_token: bool = False,
        deny_manifest_status: int | None = None,
        index: bool = True,
        host: str = "registry.test",
    ):
        self.repository = repository
        self.tag = tag
        self.platforms = platforms
        self.revision = revision
        self.deny_token = deny_token
        self.deny_manifest_status = deny_manifest_status
        self.host = host
        self.digests = {p: f"sha256:{i:064d}" for i, p in enumerate(platforms)}
        self.config_digest = "sha256:" + "c" * 64
        manifest_prefix = f"/v2/{repository}/manifests/"
        single = _json_reply(
            {
                "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                "config": {"digest": self.config_digest},
            }
        )
        index_reply = _json_reply(
            {
                "mediaType": INDEX_MEDIA_TYPE,
                "manifests": [
                    {
                        "mediaType": SINGLE_MANIFEST_MEDIA_TYPE,
                        "digest": self.digests[p],
                        "platform": {"os": p[0], "architecture": p[1]},
                    }
                    for p in platforms
                ],
            },
            content_type=INDEX_MEDIA_TYPE,
        )
        labels = (
            {"org.opencontainers.image.revision": revision}
            if revision is not None
            else {}
        )
        self.routes: dict[str, Reply] = {
            f"{manifest_prefix}{digest}": single for digest in self.digests.values()
        }
        if tag != "missing":
            self.routes[f"{manifest_prefix}{tag}"] = (
                single if not index else index_reply
            )
        self.routes[f"/v2/{repository}/blobs/{self.config_digest}"] = _json_reply(
            {"architecture": "arm64", "os": "linux", "config": {"Labels": labels}}
        )

    @property
    def ref(self) -> str:
        return f"{self.host}/{self.repository}:{self.tag}"

    def __call__(self, url: str, headers: dict[str, str]) -> Reply:
        path = url.split(self.host, 1)[1]
        if path.startswith("/token"):
            if self.deny_token:
                return 401, {}, b""
            return _json_reply({"token": "faketoken"})
        if f"/v2/{self.repository}/manifests/" in path:
            if self.deny_manifest_status is not None:
                return self.deny_manifest_status, {}, b""
            if headers.get("Authorization") != "Bearer faketoken":
                scheme = "http" if self.host.startswith("127.0.0.1:") else "https"
                challenge = (
                    f'Bearer realm="{scheme}://{self.host}/token",service="fake",'
                    f'scope="repository:{self.repository}:pull"'
                )
                return 401, {"WWW-Authenticate": challenge}, b""
        return self.routes.get(path, (404, {}, b""))
