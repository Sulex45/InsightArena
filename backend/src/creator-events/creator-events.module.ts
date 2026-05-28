import { Module } from '@nestjs/common';
import { TypeOrmModule } from '@nestjs/typeorm';
import { ContractModule } from '../contract/contract.module';
import { CreatorEvent } from '../matches/entities/creator-event.entity';
import {
  AdminCreatorEventsController,
  CreatorEventsController,
} from './creator-events.controller';
import { CreatorEventsService } from './creator-events.service';

@Module({
  imports: [ContractModule, TypeOrmModule.forFeature([CreatorEvent])],
  controllers: [CreatorEventsController, AdminCreatorEventsController],
  providers: [CreatorEventsService],
})
export class CreatorEventsModule {}
