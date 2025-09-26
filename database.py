from sqlmodel import Field, Session, SQLModel, create_engine, select
from datetime import datetime, timezone
from typing import Optional
from utils.logging import get_logger
from utils.errors import DatabaseException

class Job(SQLModel, table=True):
    id: Optional[int] = Field(default=None, primary_key=True)
    hash: str = Field(index=True, unique=True)
    title: str
    url: str
    company: str
    location: str
    description: Optional[str] = None
    score: float
    score_reasons: Optional[str] = None  # JSON string of reasons

    # Timestamps
    created_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    updated_at: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))

    # Tracking fields
    last_seen: datetime = Field(default_factory=lambda: datetime.now(timezone.utc))
    times_seen: int = Field(default=1)

    # Digest tracking
    included_in_digest: bool = Field(default=False)
    digest_sent_at: Optional[datetime] = None

    # Notification tracking
    immediate_alert_sent: bool = Field(default=False)
    alert_sent_at: Optional[datetime] = None

# The path to the SQLite database file
DB_FILE = "data/jobs.sqlite"
engine = create_engine(f"sqlite:///{DB_FILE}", echo=False)
logger = get_logger("database")

def init_db():
    """Creates the database and tables if they don't exist."""
    try:
        SQLModel.metadata.create_all(engine)
        logger.info("Database initialized successfully")
    except Exception as e:
        logger.error(f"Failed to initialize database: {e}")
        raise DatabaseException("initialization", str(e), e)

def get_job_by_hash(job_hash: str) -> Optional[Job]:
    """Checks if a job with the given hash already exists."""
    try:
        with Session(engine) as session:
            statement = select(Job).where(Job.hash == job_hash)
            return session.exec(statement).first()
    except Exception as e:
        logger.error(f"Failed to get job by hash {job_hash}: {e}")
        raise DatabaseException("get_job_by_hash", str(e), e)

def add_job(job_data: dict) -> Job:
    """Adds a new job to the database or updates if it exists."""
    try:
        with Session(engine) as session:
            # Check if job already exists
            existing_job = session.exec(
                select(Job).where(Job.hash == job_data['hash'])
            ).first()

            if existing_job:
                # Update existing job
                existing_job.last_seen = datetime.now(timezone.utc)
                existing_job.times_seen += 1
                existing_job.score = job_data.get('score', existing_job.score)
                existing_job.score_reasons = str(job_data.get('score_reasons', []))
                existing_job.updated_at = datetime.now(timezone.utc)
                session.add(existing_job)
                session.commit()
                session.refresh(existing_job)
                logger.debug(f"Updated existing job: {existing_job.title}")
                return existing_job
            else:
                # Create new job
                job = Job(
                    hash=job_data['hash'],
                    title=job_data['title'],
                    url=job_data['url'],
                    company=job_data['company'],
                    location=job_data.get('location', 'N/A'),
                    description=job_data.get('description'),
                    score=job_data['score'],
                    score_reasons=str(job_data.get('score_reasons', []))
                )
                session.add(job)
                session.commit()
                session.refresh(job)
                logger.debug(f"Added new job: {job.title}")
                return job
    except Exception as e:
        logger.error(f"Failed to add/update job {job_data.get('title', 'Unknown')}: {e}")
        raise DatabaseException("add_job", str(e), e)


def get_jobs_for_digest(min_score: float = 0.0, hours_back: int = 24) -> list[Job]:
    """Get jobs that should be included in digest, with a minimum score."""
    try:
        with Session(engine) as session:
            from datetime import timedelta
            cutoff_time = datetime.now(timezone.utc) - timedelta(hours=hours_back)

            statement = select(Job).where(
                Job.created_at >= cutoff_time,
                Job.included_in_digest == False,
                Job.score >= min_score  # Use the new min_score filter
            ).order_by(Job.score.desc(), Job.created_at.desc())

            return list(session.exec(statement).all())
    except Exception as e:
        logger.error(f"Failed to get jobs for digest: {e}")
        raise DatabaseException("get_jobs_for_digest", str(e), e)


def mark_jobs_digest_sent(job_ids: list[int]):
    """Mark jobs as included in digest."""
    try:
        with Session(engine) as session:
            statement = select(Job).where(Job.id.in_(job_ids))
            jobs = session.exec(statement).all()

            for job in jobs:
                job.included_in_digest = True
                job.digest_sent_at = datetime.now(timezone.utc)
                session.add(job)

            session.commit()
            logger.info(f"Marked {len(job_ids)} jobs as digest sent")
    except Exception as e:
        logger.error(f"Failed to mark jobs as digest sent: {e}")
        raise DatabaseException("mark_jobs_digest_sent", str(e), e)


def mark_job_alert_sent(job_id: int):
    """Mark a job as having sent an immediate alert."""
    try:
        with Session(engine) as session:
            job = session.get(Job, job_id)
            if job:
                job.immediate_alert_sent = True
                job.alert_sent_at = datetime.now(timezone.utc)
                session.add(job)
                session.commit()
    except Exception as e:
        logger.error(f"Failed to mark job {job_id} alert sent: {e}")
        raise DatabaseException("mark_job_alert_sent", str(e), e)


def get_database_stats() -> dict:
    """Get database statistics for monitoring."""
    try:
        with Session(engine) as session:
            total_jobs = len(list(session.exec(select(Job)).all()))

            # Jobs added in last 24 hours
            yesterday = datetime.now(timezone.utc).replace(hour=0, minute=0, second=0, microsecond=0)
            recent_jobs = len(list(session.exec(
                select(Job).where(Job.created_at >= yesterday)
            ).all()))

            # High score jobs (>= 0.8)
            high_score_jobs = len(list(session.exec(
                select(Job).where(Job.score >= 0.8)
            ).all()))

            return {
                "total_jobs": total_jobs,
                "recent_jobs_24h": recent_jobs,
                "high_score_jobs": high_score_jobs,
                "last_updated": datetime.now(timezone.utc).isoformat()
            }
    except Exception as e:
        logger.error(f"Failed to get database stats: {e}")
        raise DatabaseException("get_database_stats", str(e), e)


def cleanup_old_jobs(days_to_keep: int = 90):
    """Remove jobs older than specified days to manage database size."""
    try:
        from datetime import timedelta
        cutoff_date = datetime.now(timezone.utc) - timedelta(days=days_to_keep)

        with Session(engine) as session:
            old_jobs = session.exec(
                select(Job).where(Job.created_at < cutoff_date)
            ).all()

            count = len(old_jobs)
            for job in old_jobs:
                session.delete(job)

            session.commit()
            logger.info(f"Cleaned up {count} jobs older than {days_to_keep} days")
            return count
    except Exception as e:
        logger.error(f"Failed to cleanup old jobs: {e}")
        raise DatabaseException("cleanup_old_jobs", str(e), e)