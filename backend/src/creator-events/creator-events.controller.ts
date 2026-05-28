import {
  Controller,
  Get,
  Param,
  Query,
  UseGuards,
  UseInterceptors,
  ValidationPipe,
} from '@nestjs/common';
import { CacheInterceptor, CacheTTL } from '@nestjs/cache-manager';
import {
  ApiBearerAuth,
  ApiOperation,
  ApiQuery,
  ApiResponse,
  ApiTags,
} from '@nestjs/swagger';
import { Roles } from '../common/decorators/roles.decorator';
import { Role } from '../common/enums/role.enum';
import { JwtAuthGuard } from '../common/guards/jwt-auth.guard';
import { RolesGuard } from '../common/guards/roles.guard';
import { CreatorEventsService } from './creator-events.service';
import { ListParticipantsQueryDto } from './dto/list-participants-query.dto';
import { SearchEventsQueryDto } from './dto/search-events-query.dto';
import { SearchEventsResponseDto } from './dto/search-events-response.dto';

@ApiTags('creator-events')
@Controller('creator-events')
export class CreatorEventsController {
  constructor(private readonly creatorEventsService: CreatorEventsService) {}

  /**
   * GET /api/creator-events/search
   * #757 - Search creator events with relevance ranking and highlights.
   */
  @Get('search')
  @UseInterceptors(CacheInterceptor)
  @CacheTTL(120) // 2 minutes
  @ApiOperation({ summary: 'Search creator events' })
  @ApiQuery({
    name: 'q',
    required: true,
    description:
      'Search query matched against event title, description, and creator address',
  })
  @ApiQuery({ name: 'page', required: false, example: 1 })
  @ApiQuery({ name: 'limit', required: false, example: 20 })
  @ApiQuery({
    name: 'status',
    required: false,
    enum: ['active', 'cancelled', 'inactive', 'all'],
  })
  @ApiQuery({ name: 'creator', required: false })
  @ApiResponse({
    status: 200,
    description: 'Ranked creator event search results',
    type: SearchEventsResponseDto,
  })
  searchEvents(
    @Query(new ValidationPipe({ transform: true, whitelist: true }))
    query: SearchEventsQueryDto,
  ) {
    return this.creatorEventsService.searchEvents(query);
  }

  /**
   * GET /api/creator-events/:id
   * #724 — Fetch a single event by ID with enriched details.
   */
  @Get(':id')
  @UseInterceptors(CacheInterceptor)
  @CacheTTL(120) // 2 minutes
  @ApiOperation({ summary: 'Get event by ID' })
  @ApiResponse({ status: 200, description: 'Event details with enriched data' })
  @ApiResponse({ status: 404, description: 'Event not found' })
  getEvent(@Param('id') id: string) {
    return this.creatorEventsService.getEventById(id);
  }

  /**
   * GET /api/creator-events/:id/participants
   * #734 — Fetch paginated participants for an event with scores.
   */
  @Get(':id/participants')
  @UseInterceptors(CacheInterceptor)
  @CacheTTL(60) // 1 minute
  @ApiOperation({
    summary: 'Get event participants with scores and pagination',
  })
  @ApiResponse({ status: 200, description: 'Paginated participant list' })
  getParticipants(
    @Param('id') id: string,
    @Query() query: ListParticipantsQueryDto,
  ) {
    return this.creatorEventsService.getParticipants(id, query);
  }
}

@ApiTags('admin')
@Controller('admin/creator-events')
@UseGuards(JwtAuthGuard, RolesGuard)
@Roles(Role.Admin)
@ApiBearerAuth()
export class AdminCreatorEventsController {
  constructor(private readonly creatorEventsService: CreatorEventsService) {}

  /**
   * GET /api/admin/creator-events/config
   * #737 — Fetch current contract configuration (admin only).
   */
  @Get('config')
  @UseInterceptors(CacheInterceptor)
  @CacheTTL(300) // 5 minutes
  @ApiOperation({ summary: 'Get contract configuration (admin only)' })
  @ApiResponse({ status: 200, description: 'Contract configuration' })
  @ApiResponse({ status: 403, description: 'Forbidden' })
  getConfig() {
    return this.creatorEventsService.getContractConfig();
  }
}
