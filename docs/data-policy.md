# Data Policy

Data manifests describe public sources, not raw bulk data.

Rules:

- store source URLs and manifest metadata in git
- keep raw upstream files out of the repository
- write lockfiles through `just data-lock` or `just data-verify`
- prefer committed smoke fixtures for local development

