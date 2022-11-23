@if ($paginator->lastPage() > 1)
    <ul class="pagination pull-right no-margin">
        <!-- Link da p&aacute;gina anterior -->
        @if ($paginator->onFirstPage())
            {{-- <li class="disabled"><span>&laquo;</span></li> --}}
        @else
            <li><a href="{{ $paginator->previousPageUrl() }}" rel="prev">&laquo;</a></li>
        @endif

        <!-- Elementos de Pagina&ccedil;&atilde;o -->
        @foreach ($elements as $element)
            <!-- "Three Dots" Separador -->
            @if (is_string($element))
                <li class="disabled"><span>{{ $element }}</span></li>
            @endif

            <!-- Matriz De Links -->
            @if (is_array($element))
                @foreach ($element as $page => $url)
                    @if ($page == $paginator->currentPage())
                        <li class="active"><span>{{ $page }}</span></li>
                    @else
                        <li><a href="{{ $url }}">{{ $page }}</a></li>
                    @endif
                @endforeach
            @endif
        @endforeach

        <!-- Link para a pr&aacute;xima p&aacute;gina -->
        @if ($paginator->hasMorePages())
            <li><a href="{{ $paginator->nextPageUrl() }}" rel="next">&raquo;</a></li>
        @else
            {{-- <li class="disabled"><span>&raquo;</span></li> --}}
        @endif
    </ul>
@endif
