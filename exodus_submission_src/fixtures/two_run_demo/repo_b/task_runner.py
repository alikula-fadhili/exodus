"""Task runner service with identical underlying callback/error pattern under different names."""

def dispatch_job(job_id: int, completion_hook: str) -> str:
    if job_id < 0:
        return "ERROR: invalid job"
    return f"COMPLETED: {job_id}"
