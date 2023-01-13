@section('jexactyl::nav')
    <div class="row">
        <div class="col-xs-12">
            <div class="nav-tabs-custom">
                <ul class="nav nav-tabs">

                    <li @if($activeTab === 'index') class="active "@endif>
                        <a href="{{ route('admin.index') }}">Home</a>
                    </li>
                    <li @if($activeTab === 'appearance') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.appearance') }}">Aparência</a>
                    </li>
                    <li @if($activeTab === 'mail') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.mail') }}">E-mail</a>
                    </li>
                    <li @if($activeTab === 'advanced') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.advanced') }}">Avançado</a>
                    </li>

                    <li style="margin-left: 5px; margin-right: 5px;"><a>-</a></li>

                    <li @if($activeTab === 'store') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.store') }}">Loja</a>
                    </li>
                    <li @if($activeTab === 'registration') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.registration') }}">Registro</a>
                    </li>
                    <li @if($activeTab === 'approvals') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.approvals') }}">Aprovações</a>
                    </li>
                    <li @if($activeTab === 'server') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.server') }}">Configurações do servidor</a>
                    </li>
                    <li @if($activeTab === 'referrals') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.referrals') }}">Referências</a>
                        </li>
                    <li @if($activeTab === 'alerts') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.alerts') }}">Alertas</a>
                    </li>
                    <li @if($activeTab === 'coupons') class="active" @endif>
                        <a href="{{ route('admin.jexactyl.coupons') }}">Coupons</a>
                    </li>
                </ul>
            </div>
        </div>
    </div>
@endsection