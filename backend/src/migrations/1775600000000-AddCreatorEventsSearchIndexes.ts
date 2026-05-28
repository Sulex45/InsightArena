import { MigrationInterface, QueryRunner } from 'typeorm';

export class AddCreatorEventsSearchIndexes1775600000000 implements MigrationInterface {
  name = 'AddCreatorEventsSearchIndexes1775600000000';

  public async up(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(`
      CREATE INDEX IF NOT EXISTS "IDX_creator_events_search_vector"
      ON "creator_events"
      USING GIN (
        (
          setweight(to_tsvector('english', coalesce("title", '')), 'A') ||
          setweight(to_tsvector('english', coalesce("description", '')), 'B') ||
          setweight(to_tsvector('simple', coalesce("creator_address", '')), 'C')
        )
      )
    `);

    await queryRunner.query(`
      CREATE INDEX IF NOT EXISTS "IDX_creator_events_status_creator"
      ON "creator_events" ("is_active", "is_cancelled", "creator_address")
    `);
  }

  public async down(queryRunner: QueryRunner): Promise<void> {
    await queryRunner.query(
      `DROP INDEX IF EXISTS "IDX_creator_events_status_creator"`,
    );
    await queryRunner.query(
      `DROP INDEX IF EXISTS "IDX_creator_events_search_vector"`,
    );
  }
}
